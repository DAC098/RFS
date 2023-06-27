use std::str::FromStr;
use std::path::PathBuf;

use tokio::io::{AsyncWriteExt, BufWriter};
use axum::debug_handler;
use axum::http::{StatusCode, HeaderMap};
use axum::extract::{Path, Query, State, BodyStream};
use axum::response::IntoResponse;
use deadpool_postgres::GenericClient;
use tokio_postgres::types::Json as PgJson;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use lib::{ids, models};

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::util;
use crate::storage;
use crate::fs;

async fn create_directory(
    conn: &impl GenericClient,
    medium: &storage::Medium,
    id: ids::FSId,
    user_id: ids::UserId,
    parent: ids::FSId,
    basename: String,
    path: PathBuf,
    created: DateTime<Utc>,
) -> error::Result<fs::Item> {
    let storage = match &medium.type_ {
        storage::types::Type::Local(local) => {
            let mut full = local.path.join(&path);
            full.set_file_name(&basename);

            tokio::fs::create_dir(full).await?;

            storage::fs::Storage::Local(storage::fs::Local {
                id: medium.id.clone()
            })
        }
    };

    let pg_path = path.to_str().unwrap();
    let pg_storage = PgJson(&storage);

    let _ = conn.execute(
        "\
        insert into fs(\
            id, \
            user_id, \
            parent, \
            basename, \
            fs_type, \
            fs_path, \
            s_data, \
            created\
        ) values \
        ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &id,
            &user_id,
            &parent,
            &basename,
            &fs::consts::DIR_TYPE,
            &pg_path,
            &pg_storage,
            &created
        ]
    ).await?;

    Ok(fs::Item::Directory(fs::Directory {
        id,
        user_id,
        storage,
        parent,
        basename,
        path,
        tags: Default::default(),
        comment: None,
        created,
        updated: None,
        deleted: None
    }))
}

async fn stream_to_writer<W>(
    mut stream: BodyStream,
    hasher: &mut blake3::Hasher,
    writer: &mut W,
) -> error::Result<u64>
where
    W: tokio::io::AsyncWrite + Unpin
{
    use futures::StreamExt;

    let mut written: usize = 0;

    while let Some(result) = stream.next().await {
        let bytes = result?;
        let slice = bytes.as_ref();

        hasher.update(slice);

        let wrote = writer.write(slice).await?;

        if let Some(checked) = written.checked_add(wrote) {
            written = checked;
        } else {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("MaxFileSize")
                .message("the provided file is too large for the system"));
        }
    }

    writer.flush().await?;

    let size = TryFrom::try_from(written)
        .map_err(|_| error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("MaxFileSize")
            .message("the provided file is too large for the system")
            .source("total bytes written exceeds u64?"))?;

    Ok(size)
}

async fn create_file(
    conn: &impl GenericClient,
    medium: &storage::Medium,
    id: ids::FSId,
    user_id: ids::UserId,
    parent: ids::FSId,
    basename: String,
    path: PathBuf,
    mime: mime::Mime,
    created: DateTime<Utc>,
    mut stream: BodyStream,
) -> error::Result<fs::Item> {
    let size: u64;
    let hash: blake3::Hash;

    let storage = match &medium.type_ {
        storage::types::Type::Local(local) => {
            let mut full = local.path.join(&path);
            full.set_file_name(&basename);

            if full.try_exists()? {
                return Err(error::Error::new()
                    .status(StatusCode::BAD_REQUEST)
                    .kind("FileExists")
                    .message("a file exists that is unknown to the server"));
            }

            let mut hasher = blake3::Hasher::new();
            let file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(full)
                .await?;
            let mut writer = BufWriter::new(file);

            size = stream_to_writer(stream, &mut hasher, &mut writer).await?;
            hash = hasher.finalize();

            storage::fs::Storage::Local(storage::fs::Local {
                id: medium.id.clone()
            })
        }
    };

    let pg_path = path.to_str().unwrap();
    let pg_storage = PgJson(&storage);
    let pg_mime_type = mime.type_().as_str();
    let pg_mime_subtype = mime.subtype().as_str();
    let pg_size: i64 = TryFrom::try_from(size)
        .map_err(|_| error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("MaxFileSize")
            .message("the provided file is too large for the system")
            .source("total bytes written exceeds i64"))?;

    let _ = conn.execute(
        "\
        insert into fs(\
            id, \
            user_id, \
            parent, \
            basename, \
            fs_type, \
            fs_path, \
            fs_size, \
            s_data, \
            mime_type, \
            mime_subtype, \
            created\
        ) values \
        ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        &[
            &id,
            &user_id,
            &parent,
            &basename,
            &fs::consts::FILE_TYPE,
            &pg_path,
            &pg_size,
            &pg_storage,
            &pg_mime_type,
            &pg_mime_subtype,
            &created
        ]
    ).await?;

    Ok(fs::Item::File(fs::File {
        id,
        user_id,
        storage,
        parent,
        basename,
        path,
        mime,
        size,
        tags: Default::default(),
        comment: None,
        created,
        updated: None,
        deleted: None,
    }))
}

async fn update_file(
    conn: &impl GenericClient,
    medium: &storage::Medium,
    original: &mut fs::File,
    updated: DateTime<Utc>,
    stream: BodyStream
) -> error::Result<()> {
    let size: u64;
    let hash: blake3::Hash;

    match &medium.type_ {
        storage::types::Type::Local(local) => {
            let mut full = local.path.join(&original.path);
            full.set_file_name(&original.basename);

            if !full.try_exists()? {
                return Err(error::Error::new()
                    .kind("FileNotFound")
                    .message("the requested file does not exist on the system"));
            }

            let mut hasher = blake3::Hasher::new();
            let file = tokio::fs::OpenOptions::new()
                .write(true)
                .open(full)
                .await?;
            let mut writer = BufWriter::new(file);

            size = stream_to_writer(stream, &mut hasher, &mut writer).await?;
            hash = hasher.finalize();
        }
    };


    let pg_size: i64 = TryFrom::try_from(size)
        .map_err(|_| error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("MaxFileSize")
            .message("the provided file is too large for the system")
            .source("total bytes written exceeds i64"))?;

    let _ = conn.execute(
        "\
        update fs \
        set fs_size = $2, \
            updated = $3 \
        where fs.id = $1",
        &[&original.id, &pg_size, &updated]
    ).await?;

    original.updated = Some(updated);
    original.size = size;

    Ok(())
}

#[derive(Deserialize)]
pub struct PathParams {
    fs_id: ids::FSId,
}

#[derive(Deserialize)]
pub struct GetQuery {
    download: Option<bool>
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
    Query(GetQuery { download }): Query<GetQuery>
) -> error::Result<impl IntoResponse> {
    Ok(net::Json::empty())
}

#[derive(Deserialize)]
pub struct PostParams {
    fs_id: ids::FSId,
    basename: Option<String>,
}

#[derive(Deserialize)]
pub struct PostQuery {
    directory: Option<bool>,
}

#[debug_handler]
pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    headers: HeaderMap,
    Path(PostParams { fs_id, basename }): Path<PostParams>,
    Query(PostQuery { directory }): Query<PostQuery>,
    stream: BodyStream,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(item) = fs::Item::retrieve(&conn, &fs_id).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("FSNotFound")
            .message("requested fs item was not found"));
    };

    let Some(medium) = storage::Medium::retrieve(
        &conn,
        item.storage_id()
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("StorageNotFound")
            .message("requested storage item was not found"));
    };

    let is_directory = directory.unwrap_or(headers.contains_key("x-directory"));

    if item.is_file() && is_directory {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("InvalidFSItem")
            .message("cannot create directory under file"));
    }

    let transaction = conn.transaction().await?;
    let id = state.ids().wait_fs_id()?;
    let user_id = initiator.user().id().clone();
    let created = chrono::Utc::now();
    let path;
    let parent;

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
            if is_directory {
                return Err(error::Error::new()
                    .status(StatusCode::BAD_REQUEST)
                    .kind("InvalidFSItem")
                    .message("cannot create directory under file"));
            } else {
                return Err(error::Error::new()
                    .status(StatusCode::BAD_REQUEST)
                    .kind("FileExists")
                    .message("given file already exists"));
            }
        }
    }

    let basename = if let Some(value) = basename {
        value
    } else if let Some(value) = headers.get("x-basename") {
        value.to_str()?.to_owned()
    } else {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("NoBasename")
            .message("no basename was provided"));
    };

    let rtn = if is_directory {
        create_directory(
            &transaction,
            &medium,
            id,
            user_id,
            parent,
            basename,
            path,
            created
        ).await?
    } else {
        let mime = if let Some(value) = headers.get("content-type") {
            mime::Mime::from_str(value.to_str()?)?
        } else {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("NoContentType")
                .message("no content-type was specified for the file"));
        };

        create_file(
            &transaction,
            &medium,
            id,
            user_id,
            parent,
            basename,
            path,
            mime,
            created,
            stream,
        ).await?
    };

    transaction.commit().await?;

    let wrapper = lib::json::Wrapper::new(rtn.into_model());

    Ok(net::Json::new(wrapper))
}

pub async fn put(
    State(state): State<ArcShared>,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(item) = fs::Item::retrieve(&conn, &fs_id).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("FSNotFound")
            .message("requested fs item was not found"));
    };

    let Some(medium) = storage::Medium::retrieve(
        &conn,
        item.storage_id()
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("StorageNotFound")
            .message("requested storage item was not found"));
    };

    Ok(net::Json::empty())
}

pub async fn delete(
    State(state): State<ArcShared>,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    Ok(net::Json::empty())
}
