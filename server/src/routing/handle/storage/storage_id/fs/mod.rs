use std::fmt::Write;
use std::str::FromStr;
use std::path::PathBuf;

use axum::debug_handler;
use axum::http::{StatusCode, HeaderMap};
use axum::extract::{State, Path, BodyStream};
use axum::response::IntoResponse;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use lib::ids;
use lib::models::fs::{Storage, Type, ListItem, Item, File, Directory};

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::auth::initiator;
use crate::util::PgParams;
use crate::storage;
use crate::fs;

pub mod fs_id;

#[derive(Deserialize)]
pub struct PathParams {
    storage_id: ids::StorageId,
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !storage::exists_check(&conn, initiator.user().id(), &storage_id, Some(false)).await? {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("StorageNotFound")
            .message("requested storage item was not found"));
    }

    let mut search_query = format!(
        "\
        select fs.id, \
               fs.user_id, \
               fs.basename, \
               fs.fs_type, \
               fs.fs_path, \
               fs.fs_size, \
               fs.mime_type, \
               fs.mime_subtype \
        from fs \
        where fs.storage_id = $1 and \
              fs.parent is null"
    );
    let mut search_params = PgParams::with_capacity(1);
    search_params.push(&storage_id);

    let results = conn.query(search_query.as_str(), search_params.as_slice()).await?;
    let mut list = Vec::with_capacity(results.len());

    for row in results {
        let mime = if let Some(type_) = row.get::<usize, Option<&str>>(6) {
            let Some(subtype) = row.get::<usize, Option<&str>>(7) else {
                panic!("mime subtype not provided");
            };

            let mime_str = format!(
                "{}/{}",
                row.get::<usize, &str>(6),
                row.get::<usize, &str>(7),
            );

            Some(mime::Mime::from_str(mime_str.as_str()).unwrap())
        } else {
            None
        };

        list.push(ListItem {
            id: row.get(0),
            user_id: row.get(1),
            parent: None,
            basename: row.get(2),
            type_: Type::File,
            path: From::<String>::from(row.get(4)),
            size: row.get::<usize, i64>(5) as u64,
            mime
        });
    }

    Ok(net::Json::new(
        net::JsonListWrapper::with_vec(list)
    ))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    headers: HeaderMap,
    Path(PathParams { storage_id }): Path<PathParams>,
    mut stream: BodyStream,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(medium) = storage::Medium::retrieve(
        &conn,
        initiator.user().id(),
        &storage_id
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("StorageNotFound")
            .message("requested storage item was not found"));
    };

    if medium.deleted.is_some() {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("StorageNotFound")
            .message("requested storage item was not found"));
    }

    let is_directory = headers.contains_key("x-as-directory");

    let transaction = conn.transaction().await?;

    let rtn = if is_directory {
        let mut builder = fs::Directory::builder(
            state.ids().wait_fs_id()?,
            initiator.user().id().clone(),
            &medium
        );

        if let Some(value) = headers.get("x-basename") {
            builder.basename(value.to_str()?);
        }

        let dir = builder.build(&transaction).await?;

        dir.into_model_item()
    } else {
        let mut builder = fs::File::builder(
            state.ids().wait_fs_id()?,
            initiator.user().id().clone(),
            &medium
        );

        if let Some(value) = headers.get("x-basename") {
            builder.basename(value.to_str()?);
        }

        if let Some(value) = headers.get("content-type") {
            builder.mime(mime::Mime::from_str(value.to_str()?)?);
        }

        let file = builder.build(&transaction, stream).await?;

        file.into_model_item()
    };

    transaction.commit().await?;

    let wrapper = net::JsonWrapper::new(rtn);

    Ok(net::Json::new(wrapper))
}
