use lib::actions;
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::{IntoResponse};
use serde::Deserialize;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp::TotpHash;

#[derive(Deserialize)]
pub struct TotpHashParams {
    key_id: String
}

pub async fn put(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(TotpHashParams { key_id }): Path<TotpHashParams>,
    axum::Json(json): axum::Json<actions::auth::UpdateTotpHash>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(mut totp_hash) = TotpHash::retrieve_key(
        &conn,
        initiator.user().id(),
        &key_id
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("TotpHashNotFound")
            .message("requested totp hash was not found"));
    };

    if let Some(new_key) = json.key {
        totp_hash.set_key(new_key);
    }

    if json.regen {
        totp_hash.regen_hash(None)?;
    }

    let transaction = conn.transaction().await?;

    if !totp_hash.update(&transaction).await? {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("TotpHashKeyExists")
            .message("requested totp hash key already exists"));
    }

    transaction.commit().await?;

    Ok(net::Json::empty()
        .with_message("updated totp hash"))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(TotpHashParams { key_id }): Path<TotpHashParams>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(totp_hash) = TotpHash::retrieve_key(
        &conn,
        initiator.user().id(),
        &key_id
    ).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("TotpHashNotFound")
            .message("requested totp hash was not found"));
    };

    let transaction = conn.transaction().await?;

    totp_hash.delete(&transaction).await?;

    transaction.commit().await?;

    Ok(net::Json::empty()
       .with_message("deleted totp hash"))
}
