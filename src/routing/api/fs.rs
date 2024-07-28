use std::collections::HashSet;
use std::fmt::Write;
use std::io::ErrorKind as StdIoErrorKind;
use std::str::FromStr;

use rfs_lib::ids;
use rfs_api::fs::{
    DirectoryMin,
    FileMin,
    ItemMin,
    RootMin,
};

use axum::{Router, debug_handler};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, HeaderMap};
use axum::response::Response;
use axum::routing::get;
use deadpool_postgres::GenericClient;
use futures::{StreamExt, TryStreamExt};
use serde::Deserialize;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::io::ReaderStream;

use crate::error::{ApiResult, ApiError};
use crate::error::api::{Detail, Context, ApiErrorKind};
use crate::fs::{self, backend};
use crate::routing::query::PaginationQuery;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::state::ArcShared;
use crate::tags;
use crate::path;

mod storage;

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .route("/", get(retrieve))
        .route("/storage", get(storage::retrieve)
            .post(storage::create))
        .route("/storage/:storage_id", get(storage::retrieve_id)
            .patch(storage::update_id)
            .delete(storage::delete_id))
        .route("/:fs_id", get(retrieve_id)
            .post(create_item)
            .put(upload_file)
            .patch(update_item)
            .delete(delete_item))
        .route("/:fs_id/contents", get(retrieve_id_contents))
        .route("/:fs_id/download", get(download_id))
}

#[derive(Deserialize)]
pub struct PathParams {
    fs_id: ids::FSId,
}

async fn retrieve(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::FSId>>,
) -> ApiResult<rfs_api::Payload<Vec<ItemMin>>> {
    let conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![&initiator.user.id, &last_id, &fs::consts::ROOT_TYPE, &limit];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.storage_id, \
                   fs.basename, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.user_id = $1 and \
                  fs.id > $2 and \
                  fs.fs_type = $3 \
            order by fs.id \
            limit $4",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![&initiator.user.id, &fs::consts::ROOT_TYPE, &limit, &offset_num];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.storage_id, \
                   fs.basename, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.user_id = $1 and \
                  fs.fs_type = $2 \
            order by fs.id \
            limit $3 \
            offset $4",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(limit as usize);

    while let Some(row) = result.try_next().await? {
        let item = ItemMin::Root(RootMin {
            id: row.get(0),
            user_id: row.get(1),
            storage_id: row.get(2),
            basename: row.get(3),
            created: row.get(4),
            updated: row.get(5),
        });

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn retrieve_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> ApiResult<rfs_api::Payload<rfs_api::fs::Item>> {
    let conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await?;

    Ok(rfs_api::Payload::new(fs::fetch_item(&conn, &fs_id, &initiator).await?.into()))
}

#[debug_handler]
async fn create_item(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::CreateDir>,
) -> ApiResult<(StatusCode, rfs_api::Payload<rfs_api::fs::Item>)> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item(&conn, &fs_id, &initiator),
        fs::fetch_storage_from_fs_id(&conn, &fs_id),
    )?;

    let transaction = conn.transaction().await?;
    let id = state.ids().wait_fs_id()?;
    let user_id = initiator.user.id.clone();
    let storage_id = storage.id.clone();
    let created = chrono::Utc::now();
    let basename = json.basename;

    let comment = if let Some(given) = json.comment {
        if !rfs_lib::fs::comment_valid(&given) {
            return Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("comment")
            )));
        }

        Some(given)
    } else {
        None
    };

    if !rfs_lib::fs::basename_valid(&basename) {
        return Err(ApiError::from((
            ApiErrorKind::ValidationFailed,
            Detail::with_key("basename")
        )));
    }

    let Ok((parent, path, container_backend)) = item.try_into_parent_parts() else {
        return Err(ApiError::from(ApiErrorKind::InvalidType));
    };

    if fs::Item::name_check(&transaction, &parent, &basename).await?.is_some() {
        return Err(ApiError::from(ApiErrorKind::AlreadyExists));
    }

    let backend = match backend::Pair::match_up(&storage.backend, &container_backend)? {
        backend::Pair::Local((storage_local, container_local)) => {
            let mut full = storage_local.path.join(&container_local.path);
            full.push(&basename);

            tracing::debug!("new directory path: {:?}", full.display());

            tokio::fs::create_dir(&full).await?;

            backend::Node::Local(fs::backend::NodeLocal {
                path: full.strip_prefix(&storage_local.path)
                    .unwrap()
                    .to_owned()
            })
        }
    };

    tracing::debug!("directory backend: {backend:#?}");

    {
        let pg_backend = sql::ser_to_sql(&backend);

        let _ = transaction.execute(
            "\
            insert into fs(\
                id, \
                user_id, \
                storage_id, \
                parent, \
                basename, \
                fs_type, \
                fs_path, \
                backend, \
                comment, \
                created\
            ) values \
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                &id,
                &user_id,
                &storage_id,
                &parent,
                &basename,
                &fs::consts::DIR_TYPE,
                &path,
                &pg_backend,
                &comment,
                &created
            ]
        ).await?;
    }

    let tags = if let Some(tags) = json.tags {
        if !tags::validate_map(&tags) {
            return Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("tags")
            )));
        }

        tags::create_tags(&transaction, "fs_tags", "fs_id", &id, &tags).await?;

        tags
    } else {
        Default::default()
    };

    transaction.commit().await?;

    let rtn = fs::Item::Directory(fs::Directory {
        id,
        user_id,
        storage_id,
        backend,
        parent,
        basename,
        path,
        tags,
        comment,
        created,
        updated: None,
        deleted: None
    });

    Ok((StatusCode::CREATED, rfs_api::Payload::new(rtn.into())))
}

#[derive(Deserialize)]
pub struct UploadQuery {
    basename: Option<String>,
}

fn get_validation_hash(headers: &HeaderMap) -> ApiResult<Option<blake3::Hash>> {
    if let Some(hash) = headers.get("x-hash") {
        let hash_str = hash.to_str()
            .kind(ApiErrorKind::InvalidHeaderValue)?;

        let (algo, value) = hash_str.rsplit_once(':')
            .kind(ApiErrorKind::InvalidHeaderValue)?;

        match algo {
            "blake3" => {
                let hash = blake3::Hash::from_hex(value)
                    .kind(ApiErrorKind::InvalidHeaderValue)?;

                Ok(Some(hash))
            }
            _ => {
                return Err(ApiError::from(ApiErrorKind::InvalidHeaderValue));
            }
        }
    } else {
        Ok(None)
    }
}

fn get_basename(headers: &HeaderMap, query: &UploadQuery) -> ApiResult<String> {
    let found = if let Some(value) = &query.basename {
        value.clone()
    } else if let Some(value) = headers.get("x-basename") {
        value.to_str().kind_context(
            ApiErrorKind::InvalidHeaderValue,
            "x-basename contains invalid utf8 characters"
        )?.to_owned()
    } else {
        return Err(ApiError::from((
            ApiErrorKind::MissingData,
            Detail::with_key("basename")
        )));
    };

    if !rfs_lib::fs::basename_valid(&found) {
        return Err(ApiError::api((
            ApiErrorKind::ValidationFailed,
            Detail::with_key("basename")
        )));
    }

    Ok(found)
}

fn get_mime(headers: &HeaderMap) -> ApiResult<mime::Mime> {
    if let Some(value) = headers.get("content-type") {
        let content_type = value.to_str().kind_context(
            ApiErrorKind::InvalidHeaderValue,
            "content-type contains invalid utf8 characters"
        )?;

        mime::Mime::from_str(&content_type).kind_context(
            ApiErrorKind::InvalidMimeType,
            "content-type is not a valid mime format"
        )
    } else {
        Err(ApiError::from(ApiErrorKind::NoContentType))
    }
}

async fn write_file(
    id: &ids::FSId,
    full: &std::path::Path,
    tmp_dir: &std::path::Path,
    validate: Option<blake3::Hash>,
    stream: Body,
) -> ApiResult<(u64, blake3::Hash)> {
    let tmp = tmp_dir.join(format!("{}", id.id()));
    let mut written: usize = 0;
    let mut hasher = blake3::Hasher::new();

    tracing::debug!("opening tmp file: \"{}\"", tmp.display());

    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .await
        .context("failed to open tmp file for writing")?;
    let mut writer = BufWriter::new(file);

    let mut stream = stream.into_data_stream();

    while let Some(result) = stream.next().await {
        let bytes = result?;
        let slice = bytes.as_ref();

        hasher.update(slice);

        let wrote = writer.write(slice).await?;

        written = written.checked_add(wrote)
            .kind(ApiErrorKind::MaxSize)?;
    }

    writer.flush().await?;

    let size = written.try_into()
        .kind(ApiErrorKind::MaxSize)?;
    let hash = hasher.finalize();

    if let Some(validate) = validate {
        if validate != hash {
            tokio::fs::remove_file(&tmp)
                .await
                .context("failed removing tmp file after failed hash validation")?;

            return Err(ApiError::from(ApiErrorKind::InvalidHash));
        }
    }

    tracing::debug!("moving tmp to: \"{}\"", full.display());

    tokio::fs::rename(&tmp, &full)
        .await
        .context("failed to move tmp file to full path")?;

    Ok((size, hash))
}

#[debug_handler]
async fn upload_file(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    headers: HeaderMap,
    Path(PathParams { fs_id }): Path<PathParams>,
    Query(upload_query): Query<UploadQuery>,
    stream: Body,
) -> ApiResult<rfs_api::Payload<rfs_api::fs::Item>> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item(&conn, &fs_id, &initiator),
        fs::fetch_storage_from_fs_id(&conn, &fs_id),
    )?;

    let mime = get_mime(&headers)?;
    let maybe_validate = get_validation_hash(&headers)?;
    let transaction = conn.transaction().await?;

    let rtn = match item.try_into_parent_parts() {
        Ok((parent, path, container_backend)) => {
            let id = state.ids().wait_fs_id()?;
            let user_id = initiator.user.id.clone();
            let storage_id = storage.id.clone();
            let created = chrono::Utc::now();
            let basename = get_basename(&headers, &upload_query)?;

            if fs::Item::name_check(&transaction, &parent, &basename).await?.is_some() {
                return Err(ApiError::from(ApiErrorKind::AlreadyExists));
            }

            let size: u64;
            let hash: blake3::Hash;

            let backend = match backend::Pair::match_up(&storage.backend, &container_backend)? {
                backend::Pair::Local((local, node_local)) => {
                    let mut full = local.path.join(&node_local.path);
                    full.push(&basename);

                    let result = path::metadata(&full)
                        .context("failed to retrieve metadata for file")?;

                    if result.is_some() {
                        return Err(ApiError::from((
                            ApiErrorKind::AlreadyExists,
                            "an unknown file already exists in this location"
                        )));
                    }

                    let result = write_file(&id, &full, state.tmp(), maybe_validate, stream).await?;
                    size = result.0;
                    hash = result.1;

                    backend::Node::Local(fs::backend::NodeLocal {
                        path: full.strip_prefix(&local.path)
                            .unwrap()
                            .to_owned()
                    })
                }
            };

            {
                let pg_backend = sql::ser_to_sql(&backend);
                let pg_mime_type = mime.type_().as_str();
                let pg_mime_subtype = mime.subtype().as_str();
                let pg_hash = hash.as_bytes().as_slice();
                let pg_size: i64 = TryFrom::try_from(size)
                    .kind_context(ApiErrorKind::MaxSize, "total bytes written exceeds i64")?;

                let _ = transaction.execute(
                    "\
                    insert into fs(\
                        id, \
                        user_id, \
                        storage_id, \
                        parent, \
                        basename, \
                        fs_type, \
                        fs_path, \
                        fs_size, \
                        hash, \
                        backend, \
                        mime_type, \
                        mime_subtype, \
                        created\
                    ) values \
                    ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                    &[
                        &id,
                        &user_id,
                        &storage_id,
                        &parent,
                        &basename,
                        &fs::consts::FILE_TYPE,
                        &path,
                        &pg_size,
                        &pg_hash,
                        &pg_backend,
                        &pg_mime_type,
                        &pg_mime_subtype,
                        &created
                    ]
                ).await?;
            }

            fs::Item::File(fs::File {
                id,
                user_id,
                storage_id,
                backend,
                parent,
                basename,
                path,
                mime,
                size,
                hash,
                tags: Default::default(),
                comment: None,
                created,
                updated: None,
                deleted: None,
            })
        }
        Err(item) => {
            let mut file = item.into_file();
            let size: u64;
            let hash: blake3::Hash;
            let updated = chrono::Utc::now();

            if mime != file.mime {
                return Err(ApiError::from(ApiErrorKind::MimeMismatch));
            }

            match backend::Pair::match_up(&storage.backend, &file.backend)? {
                backend::Pair::Local((local, node_local)) => {
                    let full = local.path.join(&node_local.path);

                    let result = path::metadata(&full)
                        .context("failed to retrieve metadata for file")?;

                    if result.is_none() {
                        return Err(ApiError::from(ApiErrorKind::FileNotFound));
                    }

                    let result = write_file(&file.id, &full, state.tmp(), maybe_validate, stream).await?;
                    size = result.0;
                    hash = result.1;
                }
            };

            {
                let pg_hash = hash.as_bytes().as_slice();
                let pg_size: i64 = TryFrom::try_from(size)
                    .kind_context(ApiErrorKind::MaxSize, "total bytes written exceeds i64")?;

                let _ = transaction.execute(
                    "\
                    update fs \
                    set fs_size = $2, \
                        hash = $3, \
                        updated = $4 \
                    where fs.id = $1",
                    &[&file.id, &pg_size, &pg_hash, &updated]
                ).await?;
            }

            file.updated = Some(updated);
            file.size = size;
            file.hash = hash;

            fs::Item::File(file)
        }
    };

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(rtn.into()))
}

async fn update_item(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::UpdateMetadata>,
) -> ApiResult<rfs_api::Payload<rfs_api::fs::Item>> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    if !json.has_work() {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let mut item = fs::fetch_item(&conn, &fs_id, &initiator).await?;

    let transaction = conn.transaction().await?;

    {
        let updated = chrono::Utc::now();
        let mut update_query = String::from("update fs set updated = $2");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&fs_id);
        update_params.push(&updated);

        if let Some(comment) = &json.comment {
            if comment.len() == 0 {
                write!(&mut update_query, ", comment = null").unwrap();

                item.set_comment(None);
            } else {
                write!(
                    &mut update_query,
                    ", comment = ${}",
                    sql::push_param(&mut update_params, comment)
                ).unwrap();

                item.set_comment(Some(comment.clone()));
            }
        }

        write!(&mut update_query, " where id = $1").unwrap();

        transaction.execute(update_query.as_str(), update_params.as_slice()).await?;
    }

    if let Some(tags) = json.tags {
        if !tags::validate_map(&tags) {
            return Err(ApiError::from(ApiErrorKind::InvalidTags));
        }

        tags::update_tags(
            &transaction,
            "fs_tags",
            "fs_id",
            &fs_id,
            &tags
        ).await?;

        item.set_tags(tags);
    }

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(item.into()))
}

async fn delete_item(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> ApiResult<StatusCode> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item(&conn, &fs_id, &initiator),
        fs::fetch_storage_from_fs_id(&conn, &fs_id),
    )?;

    match item {
        fs::Item::Root(_root) => {
            return Err(ApiError::from(ApiErrorKind::NotPermitted));
        },
        fs::Item::Directory(dir) => {
            delete_dir(&mut conn, storage, dir).await?;
        },
        fs::Item::File(file) => {
            delete_file(&mut conn, storage, file).await?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_file(
    conn: &mut impl GenericClient,
    storage: fs::Storage,
    file: fs::File,
) -> ApiResult<()> {
    let transaction = conn.transaction().await?;

    transaction.execute(
        "delete from fs where id = $1",
        &[&file.id]
    ).await?;

    match backend::Pair::match_up(&storage.backend, &file.backend)? {
        backend::Pair::Local((local, node_local)) => {
            let full_path = local.path.join(&node_local.path);

            tokio::fs::remove_file(&full_path).await?;
        }
    }

    transaction.commit().await?;

    Ok(())
}

async fn delete_dir(
    conn: &mut impl GenericClient,
    storage: fs::Storage,
    directory: fs::Directory,
) -> ApiResult<()> {
    let transaction = conn.transaction().await?;

    let results = transaction.query_raw(
        "\
        with recursive dir_tree as (\
            select fs_root.id, \
                   fs_root.parent, \
                   fs_root.fs_type, \
                   fs_root.backend, \
                   1 as level, \
                   fs_root.hash \
            from fs fs_root \
            where id = $1 \
            union \
            select fs_contents.id, \
                   fs_contents.parent, \
                   fs_contents.fs_type, \
                   fs_contents.backend, \
                   dir_tree.level + 1 as level, \
                   fs_contents.hash \
            from fs fs_contents \
            inner join dir_tree on dir_tree.id = fs_contents.parent\
        ) \
        select * \
        from dir_tree \
        order by level desc, \
                 parent, \
                 fs_type, \
                 id",
        &[&directory.id]
    ).await?;

    futures::pin_mut!(results);

    let mut skip_parents: HashSet<ids::FSId> = HashSet::new();
    let mut deleted: Vec<ids::FSId> = Vec::new();
    let mut failed: Vec<ids::FSId> = Vec::new();
    let mut skipped: Vec<ids::FSId> = Vec::new();

    while let Some(row) = results.try_next().await? {
        let id: ids::FSId = row.get(0);
        let parent: ids::FSId = row.get(1);
        let fs_type: fs::consts::FsType = row.get(2);
        let backend: fs::backend::Node = sql::de_from_sql(row.get(3));
        let level: i32 = row.get(4);

        if skip_parents.contains(&id) {
            tracing::debug!("skipping fs item. id: {}", id.id());

            skipped.push(id);
            skip_parents.insert(parent);

            continue;
        }

        let Ok(pair) = backend::Pair::match_up(&storage.backend, &backend) else {
            tracing::error!("failed to delete item. backend miss-match. id: {}", id.id());

            failed.push(id);
            skip_parents.insert(parent);

            continue;
        };

        match pair {
            backend::Pair::Local((local, node_local)) => {
                let full_path = local.path.join(&node_local.path);

                tracing::debug!("deleting id: {}\ndepth: {level}\npath: {}", id.id(), full_path.display());

                match fs_type {
                    fs::consts::FILE_TYPE => {
                        if let Err(err) = tokio::fs::remove_file(&full_path).await {
                            match err.kind() {
                                StdIoErrorKind::NotFound => {
                                    deleted.push(id);
                                }
                                _ => {
                                    tracing::error!("failed to delete file. id: {} path: {} {err}", id.id(), full_path.display());

                                    failed.push(id);
                                    skip_parents.insert(parent);
                                }
                            }
                        } else {
                            deleted.push(id);
                        }
                    }
                    fs::consts::DIR_TYPE => {
                        if let Err(err) = tokio::fs::remove_dir(&full_path).await {
                            match err.kind() {
                                StdIoErrorKind::NotFound => {
                                    deleted.push(id);
                                }
                                _ => {
                                    tracing::error!("failed to delete directory. id: {} path: {} {err}", id.id(), full_path.display());

                                    failed.push(id);
                                    skip_parents.insert(parent);
                                }
                            }
                        } else {
                            deleted.push(id);
                        }
                    }
                    _ => {
                        tracing::debug!("unhandled file type. id: {} type: {fs_type}", id.id());

                        skipped.push(id);
                        skip_parents.insert(parent);
                    }
                }
            }
        }
    }

    let del_result = transaction.execute(
        "delete from fs where id = any($1)",
        &[&deleted]
    ).await?;

    tracing::debug!("deleted: {del_result} skipped: {} failed: {}", skipped.len(), failed.len());

    transaction.commit().await?;

    Ok(())
}

async fn retrieve_id_contents(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::FSId>>,
) -> ApiResult<rfs_api::Payload<Vec<ItemMin>>> {
    let conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read,
    ).await?;

    let item = fs::Item::retrieve(&conn, &fs_id)
        .await?
        .kind(ApiErrorKind::FileNotFound)?;

    if *item.user_id() != initiator.user.id {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    let container = item.as_container()
        .kind(ApiErrorKind::NotDirectory)?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![container.id(), &last_id, &limit];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.storage_id, \
                   fs.parent, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.parent = $1 and fs.id > $2 \
            order by fs.id \
            limit $3",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![container.id(), &limit, &offset_num];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.storage_id, \
                   fs.parent, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.parent = $1 \
            order by fs.id \
            limit $2 \
            offset $3",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(limit as usize);

    while let Some(row) = result.try_next().await? {
        let fs_type = row.get(5);

        let item = match fs_type {
            fs::consts::ROOT_TYPE => {
                ItemMin::Root(RootMin {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage_id: row.get(2),
                    basename: row.get(4),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            fs::consts::FILE_TYPE => {
                ItemMin::File(FileMin {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage_id: row.get(2),
                    parent: row.get(3),
                    basename: row.get(4),
                    path: row.get(6),
                    size: sql::u64_from_sql(row.get(7)),
                    mime: sql::mime_from_sql(row.get(8), row.get(9)),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            fs::consts::DIR_TYPE => {
                ItemMin::Directory(DirectoryMin {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage_id: row.get(2),
                    parent: row.get(3),
                    basename: row.get(4),
                    path: row.get(6),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            _ => {
                panic!("unexpected fs_type when retrieving fs item. type: {fs_type}");
            }
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

async fn download_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> ApiResult<Response<Body>> {
    let conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item(&conn, &fs_id, &initiator),
        fs::fetch_storage_from_fs_id(&conn, &fs_id),
    )?;

    let Ok(file): Result<fs::File, _> = item.try_into() else {
        return Err(ApiError::from(ApiErrorKind::NotFile));
    };

    let builder = Response::builder()
        .status(StatusCode::OK)
        .header("content-disposition", format!("attachment; filename=\"{}\"", file.basename))
        .header("content-type", file.mime.to_string())
        .header("content-length", file.size)
        .header("x-checksum", format!("blake3:{}", file.hash));

    match backend::Pair::match_up(&storage.backend, &file.backend)? {
        backend::Pair::Local((local, node_local)) => {
            let full = local.path.join(&node_local.path);
            let stream = ReaderStream::new(OpenOptions::new()
                .read(true)
                .open(full)
                .await?);

            Ok(builder.body(Body::from_stream(stream))?)
        }
    }
}
