use rfs_lib::actions;
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::{IntoResponse};
use serde::Deserialize;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp;

#[derive(Deserialize)]
pub struct PathParams {
    key_id: String
}

pub async fn patch(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(PathParams { key_id }): Path<PathParams>,
    axum::Json(json): axum::Json<actions::auth::UpdateTotpHash>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(mut hash) = totp::recovery::Hash::retrieve_key(
        &conn,
        initiator.user().id(),
        &key_id
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("KeyNotFound")
            .message("requested totp recovery hash was not found"));
    };

    if let Some(new_key) = json.key {
        if !rfs_lib::sec::authn::totp::recovery::key_valid(&new_key) {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidKey")
                .message("the provided key is an invalid format"));
        }

        if totp::recovery::key_exists(&conn, initiator.user().id(), &new_key).await? {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("KeyExits")
                .message("the provided key already exists"));
        }

        hash.set_key(new_key);
    }

    if json.regen {
        hash.regen_hash()?;
    }

    let transaction = conn.transaction().await?;

    hash.update(&transaction).await?;

    transaction.commit().await?;

    Ok(net::Json::empty()
        .with_message("updated totp hash"))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(PathParams { key_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(hash) = totp::recovery::Hash::retrieve_key(
        &conn,
        initiator.user().id(),
        &key_id
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("KeyNotFound")
            .message("requested totp recovery hash was not found"));
    };

    let transaction = conn.transaction().await?;

    hash.delete(&transaction).await?;

    transaction.commit().await?;

    Ok(net::Json::empty()
        .with_message("deleted totp hash"))
}
