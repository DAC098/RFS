use std::fmt::Write;

use rfs_lib::ids;
use rfs_lib::actions::storage::{UpdateStorage, UpdateStorageType};
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::util::sql;
use crate::storage;
use crate::tags;

//pub mod root;

#[derive(Deserialize)]
pub struct PathParams {
    storage_id: ids::StorageId,
}

pub async fn get(
    State(state): State<ArcShared>,
    _initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    let Some(medium) = storage::Medium::retrieve(
        &conn,
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

    let rtn = rfs_lib::json::Wrapper::new(medium.into_schema());

    Ok(net::Json::new(rtn))
}

pub async fn put(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
    axum::Json(json): axum::Json<UpdateStorage>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(mut medium) = storage::Medium::retrieve(
        &conn,
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

    if !json.has_work() {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("NoWork")
            .message("requested update with no changes"));
    }

    let transaction = conn.transaction().await?;

    if json.name.is_some() || json.type_.is_some() {
        let updated = chrono::Utc::now();
        let mut update_query = String::from("update storage set updated = $2");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&storage_id);
        update_params.push(&updated);

        if let Some(name) = &json.name {
            if let Some(found_id) = storage::name_check(&transaction, initiator.user().id(), name).await? {
                if found_id != storage_id {
                    return Err(error::Error::new()
                        .status(StatusCode::BAD_REQUEST)
                        .kind("StorageNameExists")
                        .message("requested storage name already exists"));
                }
            }

            write!(
                &mut update_query,
                "name = ${} ",
                sql::push_param(&mut update_params, name)
            ).unwrap();

            medium.name = name.clone();
        }

        if let Some(type_) = &json.type_ {
            match type_ {
                UpdateStorageType::Local {..} => {}
            }
        }

        write!(&mut update_query, "where storage_id = $1").unwrap();

        transaction.execute(update_query.as_str(), update_params.as_slice()).await?;
    }

    if let Some(tags) = json.tags {
        tags::update_tags(
            &transaction,
            "storage_tags",
            "storage_id",
            &storage_id,
            &tags
        ).await?;

        medium.tags = tags;
    }

    transaction.commit().await?;

    let rtn = rfs_lib::json::Wrapper::new(medium.into_schema())
        .with_message("updated storage");

    Ok(net::Json::new(rtn))
}

pub async fn delete(
    State(state): State<ArcShared>,
    _initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(_medium) = storage::Medium::retrieve(
        &conn,
        &storage_id
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("StorageNotFound")
            .message("requested storage item was not found"));
    };

    let deleted = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    // soft delete fs items
    let _ = transaction.execute(
        "update fs set deleted = $2 where storage_id = $1",
        &[&storage_id, &deleted]
    ).await?;

    // soft delete storage item
    let _ = transaction.execute(
        "update storage set deleted = $2 where storage_id = $1",
        &[&storage_id, &deleted]
    ).await?;

    let body = rfs_lib::json::Wrapper::new(())
        .with_message("deleted storage");

    Ok(net::Json::new(body))
}
