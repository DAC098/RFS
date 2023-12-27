use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::{Deserialize};
use rfs_lib::ids;

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::storage;
use crate::fs;

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

    let Some(root) = fs::Root::storage_id_retrieve(&conn, &storage_id).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("FSItemNotFound")
            .message("storage medium root was not found"));
    };

    tracing::event!(
        tracing::Level::DEBUG,
        "storage root: {:?}",
        root
    );

    Ok(net::Json::empty())
}
