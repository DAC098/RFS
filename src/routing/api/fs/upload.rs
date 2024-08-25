use std::str::FromStr;

use rfs_lib::ids;

use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::HeaderMap;
use deadpool_postgres::GenericClient;
use futures::StreamExt;
use serde::Deserialize;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::error::{ApiResult, ApiError};
use crate::error::api::{Detail, Context, ApiErrorKind};
use crate::fs::{self, backend};
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::path;
use crate::db;

#[derive(Deserialize)]
pub struct PathParams {
    fs_uid: ids::FSUid,
}

#[derive(Deserialize)]
pub struct UploadQuery {
    basename: Option<String>,
}

pub async fn upload_file(
    db::Conn(mut conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    headers: HeaderMap,
    Path(PathParams { fs_uid }): Path<PathParams>,
    Query(upload_query): Query<UploadQuery>,
    stream: Body,
) -> ApiResult<rfs_api::Payload<rfs_api::fs::Item>> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item_uid(&conn, &fs_uid, &initiator),
        fs::fetch_storage_from_fs_uid(&conn, &fs_uid),
    )?;

    let mime = get_mime(&headers)?;
    let maybe_validate = get_validation_hash(&headers)?;
    let transaction = conn.transaction().await?;

    let rtn = match item.try_into_parent_parts() {
        Ok((parent, path, container_backend)) => {
            let uid = ids::FSUid::gen();
            let user = initiator.user.id.clone();
            let storage_id = storage.id.clone();
            let created = chrono::Utc::now();
            let basename = get_basename(&headers, &upload_query)?;

            if fs::Item::name_check(&transaction, parent.local(), &basename).await?.is_some() {
                return Err(ApiError::from(ApiErrorKind::AlreadyExists));
            }

            match backend::Pair::match_up(&storage.backend, &container_backend)? {
                backend::Pair::Local((local, node_local)) => {
                    let dir = local.path.join(&node_local.path);
                    let full = dir.join(&basename);
                    let tmp = dir.join(format!("{}.tmp.rfs", uid));

                    tracing::debug!("tmp path: \"{}\"", tmp.display());

                    let result = path::metadata(&full)
                        .context("failed to retrieve metadata for file")?;

                    if result.is_some() {
                        return Err(ApiError::from((
                            ApiErrorKind::AlreadyExists,
                            "an unknown file already exists in this location"
                        )));
                    }

                    let tmp_file = create_file(&tmp).await?;

                    let (size, hash) = match write_body(tmp_file, maybe_validate, stream).await {
                        Ok(result) => result,
                        Err(err) => {
                            tokio::fs::remove_file(&tmp)
                                .await
                                .context("failed removing tmp file after failed hash validation")?;

                            return Err(err);
                        }
                    };

                    let backend = backend::Node::Local(fs::backend::NodeLocal {
                        path: full.strip_prefix(&local.path)
                            .unwrap()
                            .to_owned()
                    });

                    let tmp_id = ids::FSId::try_from(1).unwrap();

                    let mut file = fs::File {
                        id: ids::FSSet::new(tmp_id, uid),
                        user,
                        storage: storage_id,
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
                    };

                    if let Err(err) = insert_file(&mut file, &transaction).await {
                        tokio::fs::remove_file(&tmp)
                            .await
                            .context("failed removing tmp file after inserting database")?;

                        return Err(err);
                    }

                    if let Err(err) = tokio::fs::rename(&tmp, &full).await {
                        tokio::fs::remove_file(&tmp)
                            .await
                            .context("failed removing tmp file after failed hash validation")?;

                        return Err(ApiError::from(err)
                            .context("failed to move tmp file to full path"));
                    }

                    if let Err(err) = transaction.commit().await {
                        tokio::fs::remove_file(&full)
                            .await
                            .context("failed to remove full after committing database")?;

                        return Err(ApiError::from(err));
                    }

                    fs::Item::File(file)
                }
            }
        }
        Err(item) => {
            let mut file = item.into_file();
            file.mime = mime;

            match backend::Pair::match_up(&storage.backend, &file.backend)? {
                backend::Pair::Local((local, node_local)) => {
                    let full = local.path.join(&node_local.path);
                    let parent_dir = full.parent()
                        .context("failed to retrieve parent directory of file?")?;
                    let tmp = parent_dir.join(format!("{}.tmp.rfs", file.id.uid()));
                    let prev = parent_dir.join(format!("{}.prev.rfs", file.id.uid()));

                    tracing::debug!("tmp path: \"{}\"", tmp.display());
                    tracing::debug!("prev path: \"{}\"", prev.display());

                    let result = path::metadata(&full)
                        .context("failed to retrieve metadata for file")?;

                    if result.is_none() {
                        return Err(ApiError::from(ApiErrorKind::FileNotFound));
                    }

                    let tmp_file = create_file(&tmp).await?;

                    let (size, hash) = match write_body(tmp_file, maybe_validate, stream).await {
                        Ok(result) => result,
                        Err(err) => {
                            tokio::fs::remove_file(&tmp)
                                .await
                                .context("failed removing tmp file after failed hash validation")?;

                            return Err(err);
                        }
                    };

                    file.size = size;
                    file.hash = hash;
                    file.updated = Some(chrono::Utc::now());

                    if let Err(err) = update_file(&file, &transaction).await {
                        tokio::fs::remove_file(&tmp)
                            .await
                            .context("failed removing tmp file after updating database")?;

                        return Err(err);
                    }

                    // now begins the dance of file updates

                    // first move the current file to the prev
                    if let Err(err) = tokio::fs::rename(&full, &prev).await {
                        // try to remove the tmp file
                        tokio::fs::remove_file(&tmp)
                            .await
                            .context("failed removing tmp after failed moving full to prev")?;

                        return Err(ApiError::from(err)
                            .context("failed to move full to prev"));
                    }

                    // then move the tmp file to full
                    if let Err(err) = tokio::fs::rename(&tmp, &full).await {
                        // try to move the prev file back to the original
                        tokio::fs::rename(&prev, &full)
                            .await
                            .context("failed to move prev to full after moving tmp to full")?;

                        // try to remove the tmp file, since this operation did
                        // not succeed the tmp file should still be in its
                        // original path
                        tokio::fs::remove_file(&tmp)
                            .await
                            .context("failed removing tmp after failed moving full to prev")?;

                        return Err(ApiError::from(err)
                            .context("failed to move tmp to full"));
                    }

                    // commit to the database
                    if let Err(err) = transaction.commit().await {
                        // since the tmp file was moved to the full path we
                        // need to try and move the prev file back to the full
                        // path
                        tokio::fs::rename(&prev, &full)
                            .await
                            .context("failed to move prev to full after committing database")?;

                        return Err(ApiError::from(err));
                    }

                    // remove the prev file as everything else succeeded but if
                    // this errors then the system is still good as the new
                    // file is in the correct position and the database has
                    // been updated
                    tokio::fs::remove_file(&prev)
                        .await
                        .context("failed to remove prev file")?;

                    fs::Item::File(file)
                }
            }
        }
    };

    Ok(rfs_api::Payload::new(rtn.into()))
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

async fn create_file(path: &std::path::Path) -> ApiResult<BufWriter<tokio::fs::File>> {
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .context("failed to open file for writing")?;

    Ok(BufWriter::new(file))
}

async fn write_body<T>(
    mut writer: T,
    validate: Option<blake3::Hash>,
    stream: Body,
) -> ApiResult<(u64, blake3::Hash)>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    let mut written: usize = 0;
    let mut hasher = blake3::Hasher::new();

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
            return Err(ApiError::from(ApiErrorKind::InvalidHash));
        }
    }

    Ok((size, hash))
}

async fn insert_file(file: &mut fs::File, conn: &impl GenericClient) -> ApiResult<()> {
    let id = {
        let pg_backend = sql::ser_to_sql(&file.backend);
        let pg_mime_type = file.mime.type_().as_str();
        let pg_mime_subtype = file.mime.subtype().as_str();
        let pg_hash = file.hash.as_bytes().as_slice();
        let pg_size: i64 = TryFrom::try_from(file.size)
            .kind_context(ApiErrorKind::MaxSize, "total bytes written exceeds i64")?;

        let result = conn.query_one(
            "\
            insert into fs(\
                uid, \
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
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
            returing id",
            &[
                file.id.uid(),
                file.user.local(),
                file.storage.local(),
                file.parent.local(),
                &file.basename,
                &fs::consts::FILE_TYPE,
                &file.path,
                &pg_size,
                &pg_hash,
                &pg_backend,
                &pg_mime_type,
                &pg_mime_subtype,
                &file.created
            ]
        ).await?;

        result.get(0)
    };

    file.id = ids::FSSet::new(id, file.id.uid().clone());

    Ok(())
}

async fn update_file(file: &fs::File, conn: &impl GenericClient) -> ApiResult<()> {
    let pg_hash = file.hash.as_bytes().as_slice();
    let pg_size: i64 = TryFrom::try_from(file.size)
        .kind_context(ApiErrorKind::MaxSize, "total bytes written exceeds i64")?;

    let _ = conn.execute(
        "\
        update fs \
        set fs_size = $2, \
            hash = $3, \
            updated = $4 \
        where fs.id = $1",
        &[file.id.local(), &pg_size, &pg_hash, &file.updated]
    ).await?;

    Ok(())
}
