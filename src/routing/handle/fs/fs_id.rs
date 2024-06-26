use std::collections::HashSet;
use std::fmt::Write;
use std::str::FromStr;
use std::io::ErrorKind as StdIoErrorKind;

use rfs_lib::ids;
use tokio::io::{AsyncWriteExt, BufWriter};
use axum::debug_handler;
use axum::http::{StatusCode, HeaderMap};
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use futures::{Stream, StreamExt, TryStreamExt};
use deadpool_postgres::GenericClient;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::fs;
use crate::tags;

pub mod contents;
pub mod dl;

async fn stream_to_writer<S, E, W>(
    mut stream: S,
    hasher: &mut blake3::Hasher,
    writer: &mut W,
) -> error::Result<u64>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
    error::Error: From<E>,
{
    let mut written: usize = 0;

    while let Some(result) = stream.next().await {
        let bytes = result?;
        let slice = bytes.as_ref();

        hasher.update(slice);

        let wrote = writer.write(slice).await?;

        if let Some(checked) = written.checked_add(wrote) {
            written = checked;
        } else {
            return Err(error::Error::api(error::ApiErrorKind::MaxSize));
        }
    }

    writer.flush().await?;

    let Ok(size) = TryFrom::try_from(written) else {
        return Err(error::Error::api(error::ApiErrorKind::MaxSize));
    };

    Ok(size)
}

#[derive(Deserialize)]
pub struct PathParams {
    fs_id: ids::FSId,
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> error::Result<rfs_api::Payload<rfs_api::fs::Item>> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    Ok(rfs_api::Payload::new(fs::fetch_item(&conn, &fs_id, &initiator).await?.into()))
}

#[debug_handler]
pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::CreateDir>,
) -> error::Result<(StatusCode, rfs_api::Payload<rfs_api::fs::Item>)> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let item = fs::fetch_item(&conn, &fs_id, &initiator).await?;
    let storage = fs::fetch_storage(&conn, item.storage_id()).await?;

    let transaction = conn.transaction().await?;
    let id = state.ids().wait_fs_id()?;
    let user_id = initiator.user.id.clone();
    let storage_id = storage.id.clone();
    let created = chrono::Utc::now();
    let basename = json.basename;

    let comment = if let Some(given) = json.comment {
        if !rfs_lib::fs::comment_valid(&given) {
            return Err(error::Error::api((
                error::ApiErrorKind::ValidationFailed,
                error::Detail::with_key("comment")
            )));
        }

        Some(given)
    } else {
        None
    };

    if !rfs_lib::fs::basename_valid(&basename) {
        return Err(error::Error::api((
            error::ApiErrorKind::ValidationFailed,
            error::Detail::with_key("basename")
        )));
    }

    let Ok((parent, path, container_backend)) = item.try_into_parent_parts() else {
        return Err(error::Error::api(error::ApiErrorKind::InvalidType));
    };

    if fs::Item::name_check(&transaction, &parent, &basename).await?.is_some() {
        return Err(error::Error::api(error::ApiErrorKind::AlreadyExists));
    }

    let backend = match fs::backend::Pair::match_up(&storage.backend, &container_backend)? {
        fs::backend::Pair::Local((storage_local, container_local)) => {
            let mut full = storage_local.path.join(&container_local.path);
            full.push(&basename);

            tracing::debug!("new directory path: {:?}", full.display());

            tokio::fs::create_dir(&full).await?;

            fs::backend::Node::Local(fs::backend::NodeLocal {
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
            return Err(error::Error::api((
                error::ApiErrorKind::ValidationFailed,
                error::Detail::with_key("tags")
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
pub struct PutQuery {
    basename: Option<String>,
}

#[debug_handler]
pub async fn put(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    headers: HeaderMap,
    Path(PathParams { fs_id }): Path<PathParams>,
    Query(PutQuery { basename }): Query<PutQuery>,
    stream: Body,
) -> error::Result<rfs_api::Payload<rfs_api::fs::Item>> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(
            error::ApiErrorKind::PermissionDenied
        ));
    }

    let item = fs::fetch_item(&conn, &fs_id, &initiator).await?;
    let storage = fs::fetch_storage(&conn, item.storage_id()).await?;

    let mime = if let Some(value) = headers.get("content-type") {
        mime::Mime::from_str(value.to_str()?)?
    } else {
        return Err(error::Error::api(error::ApiErrorKind::NoContentType));
    };

    let transaction = conn.transaction().await?;

    let rtn = match item.try_into_parent_parts() {
        Ok((parent, path, container_backend)) => {
            let id = state.ids().wait_fs_id()?;
            let user_id = initiator.user.id.clone();
            let storage_id = storage.id.clone();
            let created = chrono::Utc::now();

            let basename = if let Some(value) = basename {
                value
            } else if let Some(value) = headers.get("x-basename") {
                value.to_str()?.to_owned()
            } else {
                return Err(error::Error::api((
                    error::ApiErrorKind::MissingData,
                    error::Detail::with_key("basename")
                )));
            };

            if !rfs_lib::fs::basename_valid(&basename) {
                return Err(error::Error::api((
                    error::ApiErrorKind::ValidationFailed,
                    error::Detail::with_key("basename")
                )));
            }

            if fs::Item::name_check(&transaction, &parent, &basename).await?.is_some() {
                return Err(error::Error::api(error::ApiErrorKind::AlreadyExists));
            }

            let size: u64;
            let hash: blake3::Hash;

            let backend = match fs::backend::Pair::match_up(&storage.backend, &container_backend)? {
                fs::backend::Pair::Local((local, node_local)) => {
                    let mut full = local.path.join(&node_local.path);
                    full.push(&basename);

                    tracing::debug!("file create path: {}", full.display());

                    if full.try_exists()? {
                        return Err(error::Error::api((
                            error::ApiErrorKind::AlreadyExists,
                            "an unknown file already exists in this location"
                        )));
                    }

                    let mut hasher = blake3::Hasher::new();
                    let file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&full)
                        .await?;
                    let mut writer = BufWriter::new(file);

                    size = stream_to_writer(stream.into_data_stream(), &mut hasher, &mut writer).await?;
                    hash = hasher.finalize();

                    fs::backend::Node::Local(fs::backend::NodeLocal {
                        path: full.strip_prefix(&local.path)
                            .unwrap()
                            .to_owned()
                    })
                }
            };

            tracing::debug!("file backend: {backend:#?}");

            {
                let pg_backend = sql::ser_to_sql(&backend);
                let pg_mime_type = mime.type_().as_str();
                let pg_mime_subtype = mime.subtype().as_str();
                let pg_hash = hash.as_bytes().as_slice();
                let Ok(pg_size): Result<i64, _> = TryFrom::try_from(size) else {
                    return Err(error::Error::api(error::ApiErrorKind::MaxSize)
                        .context("total bytes written exceeds i64"));
                };

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
                return Err(error::Error::api(error::ApiErrorKind::MimeMismatch));
            }

            match &storage.backend {
                fs::backend::Config::Local(storage_backend) => {
                    let full = {
                        let Some(file_backend) = file.backend.as_local() else {
                            return Err(error::Error::new().context("backend miss-match when updating file"));
                        };

                        storage_backend.path.join(&file_backend.path)
                    };

                    if !full.try_exists()? {
                        return Err(error::Error::api(error::ApiErrorKind::FileNotFound));
                    }

                    tracing::debug!("file update path: {}", full.display());

                    let mut hasher = blake3::Hasher::new();
                    let file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .open(full)
                        .await?;
                    let mut writer = BufWriter::new(file);

                    size = stream_to_writer(
                        stream.into_data_stream(),
                        &mut hasher,
                        &mut writer
                    ).await?;
                    hash = hasher.finalize();
                }
            };

            {
                let pg_hash = hash.as_bytes().as_slice();
                let Ok(pg_size): Result<i64, _> = TryFrom::try_from(size) else {
                    return Err(error::Error::api(error::ApiErrorKind::MaxSize)
                        .context("total bytes written exceeds i64"));
                };

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

pub async fn patch(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::UpdateMetadata>,
) -> error::Result<rfs_api::Payload<rfs_api::fs::Item>> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    if !json.has_work() {
        return Err(error::Error::api(error::ApiErrorKind::NoWork));
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
            return Err(error::Error::api(error::ApiErrorKind::InvalidTags));
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

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> error::Result<StatusCode> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let item = fs::fetch_item(&conn, &fs_id, &initiator).await?;
    let storage = fs::fetch_storage(&conn, item.storage_id()).await?;

    match item {
        fs::Item::Root(_root) => {
            return Err(error::Error::api(error::ApiErrorKind::NotPermitted));
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
) -> error::Result<()> {
    let transaction = conn.transaction().await?;

    transaction.execute(
        "delete from fs where id = $1",
        &[&file.id]
    ).await?;

    match fs::backend::Pair::match_up(&storage.backend, &file.backend)? {
        fs::backend::Pair::Local((local, node_local)) => {
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
) -> error::Result<()> {
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

        let Ok(pair) = fs::backend::Pair::match_up(&storage.backend, &backend) else {
            tracing::error!("failed to delete item. backend miss-match. id: {}", id.id());

            failed.push(id);
            skip_parents.insert(parent);

            continue;
        };

        match pair {
            fs::backend::Pair::Local((local, node_local)) => {
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
