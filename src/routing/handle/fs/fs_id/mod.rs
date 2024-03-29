use std::fmt::Write;
use std::str::FromStr;
use std::path::PathBuf;

use rfs_lib::ids;
use tokio::io::{AsyncWriteExt, BufWriter};
use axum::debug_handler;
use axum::http::{StatusCode, HeaderMap};
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures::{Stream, StreamExt};
use deadpool_postgres::GenericClient;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::storage;
use crate::fs;
use crate::tags;

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
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(item) = fs::Item::retrieve(&conn,&fs_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::FileNotFound));
    };

    if *item.user_id() != initiator.user.id {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    Ok(rfs_api::Payload::new(item.into_schema()))
}

#[debug_handler]
pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::CreateDir>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(item) = fs::Item::retrieve(&conn, &fs_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::FileNotFound));
    };

    let Some(medium) = storage::Medium::retrieve(&conn, item.storage_id()).await? else {
        return Err(error::Error::api(error::ApiErrorKind::StorageNotFound));
    };

    let transaction = conn.transaction().await?;
    let id = state.ids().wait_fs_id()?;
    let user_id = initiator.user().id().clone();
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

    let path;
    let parent;

    if !rfs_lib::fs::basename_valid(&basename) {
        return Err(error::Error::api((
            error::ApiErrorKind::ValidationFailed,
            error::Detail::with_key("basename")
        )));
    }

    match item {
        fs::Item::Root(root) => {
            path = PathBuf::new();
            parent = root.id.clone();
        },
        fs::Item::Directory(dir) => {
            path = dir.path.join(&dir.basename);
            parent = dir.id.clone();
        },
        fs::Item::File(_) => {
            return Err(error::Error::api(
                error::ApiErrorKind::InvalidType
            ));
        }
    }

    let storage = match &medium.type_ {
        storage::types::Type::Local(local) => {
            let mut full = local.path.join(&path);
            full.push(&basename);

            tracing::debug!(
                "new directory path: {:?}",
                full.display()
            );

            tokio::fs::create_dir(full).await?;

            storage::fs::Storage::Local(storage::fs::Local {
                id: medium.id.clone()
            })
        }
    };

    {
        let pg_path = path.to_str().unwrap();
        let pg_storage = sql::ser_to_sql(&storage);

        let _ = transaction.execute(
            "\
            insert into fs(\
                id, \
                user_id, \
                parent, \
                basename, \
                fs_type, \
                fs_path, \
                s_data, \
                comment, \
                created\
            ) values \
            ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            &[
                &id,
                &user_id,
                &parent,
                &basename,
                &fs::consts::DIR_TYPE,
                &pg_path,
                &pg_storage,
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
        storage,
        parent,
        basename,
        path,
        tags,
        comment,
        created,
        updated: None,
        deleted: None
    });

    Ok((StatusCode::CREATED, rfs_api::Payload::new(rtn.into_schema())))
}

#[derive(Deserialize)]
pub struct PutQuery {
    basename: Option<String>,
    overwrite: Option<bool>,
}

#[debug_handler]
pub async fn put(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    headers: HeaderMap,
    Path(PathParams { fs_id }): Path<PathParams>,
    Query(PutQuery { basename, overwrite: _ }): Query<PutQuery>,
    stream: Body,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(
            error::ApiErrorKind::PermissionDenied
        ));
    }

    let Some(item) = fs::Item::retrieve(&conn, &fs_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::FileNotFound));
    };

    if item.user_id() != initiator.user().id() {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    tracing::debug!(
        "retrieved fs item: {:#?}",
        item
    );

    let Some(medium) = storage::Medium::retrieve(
        &conn,
        item.storage_id()
    ).await? else {
        return Err(error::Error::api(error::ApiErrorKind::StorageNotFound));
    };

    let transaction = conn.transaction().await?;
    let created = chrono::Utc::now();

    let mime = if let Some(value) = headers.get("content-type") {
        mime::Mime::from_str(value.to_str()?)?
    } else {
        return Err(error::Error::api(error::ApiErrorKind::NoContentType));
    };

    let rtn = if !item.is_file() {
        let id = state.ids().wait_fs_id()?;
        let user_id = initiator.user().id().clone();
        let size: u64;
        let hash: blake3::Hash;
        let path: PathBuf;
        let parent;

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

        if let Some(_id) = fs::name_check(&transaction, item.id(), &basename).await? {
            return Err(error::Error::api(error::ApiErrorKind::AlreadyExists));
        }

        match item {
            fs::Item::Root(root) => {
                path = PathBuf::new();
                parent = root.id.clone();
            },
            fs::Item::Directory(dir) => {
                path = dir.path.join(&dir.basename);
                parent = dir.id.clone();
            },
            fs::Item::File(_) => unreachable!()
        }

        let storage = match &medium.type_ {
            storage::types::Type::Local(local) => {
                let mut full = local.path.join(&path);
                full.push(&basename);

                tracing::debug!(
                    "new file path: {:?}",
                    full.display()
                );

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
                    .open(full)
                    .await?;
                let mut writer = BufWriter::new(file);

                size = stream_to_writer(stream.into_data_stream(), &mut hasher, &mut writer).await?;
                hash = hasher.finalize();

                storage::fs::Storage::Local(storage::fs::Local {
                    id: medium.id.clone()
                })
            }
        };

        {
            let pg_path = path.to_str().unwrap();
            let pg_storage = sql::ser_to_sql(&storage);
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
                    parent, \
                    basename, \
                    fs_type, \
                    fs_path, \
                    fs_size, \
                    hash, \
                    s_data, \
                    mime_type, \
                    mime_subtype, \
                    created\
                ) values \
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                &[
                    &id,
                    &user_id,
                    &parent,
                    &basename,
                    &fs::consts::FILE_TYPE,
                    &pg_path,
                    &pg_size,
                    &pg_hash,
                    &pg_storage,
                    &pg_mime_type,
                    &pg_mime_subtype,
                    &created
                ]
            ).await?;
        }

        fs::Item::File(fs::File {
            id,
            user_id,
            storage,
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
    } else {
        let mut file = item.into_file();
        let size: u64;
        let hash: blake3::Hash;

        if mime != file.mime {
            return Err(error::Error::api(error::ApiErrorKind::MimeMismatch));
        }

        match &medium.type_ {
            storage::types::Type::Local(local) => {
                let mut full = local.path.join(&file.path);
                full.push(&file.basename);

                if !full.try_exists()? {
                    return Err(error::Error::api(error::ApiErrorKind::FileNotFound));
                }

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
                &[&file.id, &pg_size, &pg_hash, &created]
            ).await?;
        }

        file.updated = Some(created);
        file.size = size;
        file.hash = hash;

        fs::Item::File(file)
    };

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(rtn.into_schema()))
}

pub async fn patch(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::UpdateMetadata>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(mut item) = fs::Item::retrieve(
        &conn,
        &fs_id
    ).await? else {
        return Err(error::Error::api(error::ApiErrorKind::FileNotFound));
    };

    if item.user_id() != initiator.user().id() {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    if !json.has_work() {
        return Err(error::Error::api(error::ApiErrorKind::NoWork));
    }

    tracing::debug!("action {:?}", json);

    let transaction = conn.transaction().await?;

    {
        let updated = chrono::Utc::now();
        let mut update_query = String::from("update fs set updated = $2");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&fs_id);
        update_params.push(&updated);

        if let Some(comment) = &json.comment {
            if comment.len() == 0 {
                write!(
                    &mut update_query,
                    ", comment = null",
                ).unwrap();

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

    Ok(rfs_api::Payload::new(item.into_schema()))
}

pub async fn delete(
    State(_state): State<ArcShared>,
    _initiator: initiator::Initiator,
    Path(PathParams { fs_id: _ }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    Ok(StatusCode::NO_CONTENT)
}
