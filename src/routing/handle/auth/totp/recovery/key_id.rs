use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp;

#[derive(Deserialize)]
pub struct PathParams {
    key_id: String
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(PathParams { key_id }): Path<PathParams>
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    let Some(hash) = totp::recovery::Hash::retrieve_key(
        &conn,
        &initiator.user.id,
        &key_id
    ).await? else {
        return Err(error::Error::api(error::AuthKind::TotpRecoveryNotFound));
    };

    Ok(rfs_api::Payload::new(rfs_api::auth::totp::TotpRecovery {
        user_id: hash.user_id,
        key: hash.key.into(),
        hash: hash.hash.into(),
        used: hash.used.into(),
    }))
}

pub async fn patch(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(PathParams { key_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::auth::totp::UpdateTotpHash>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(mut hash) = totp::recovery::Hash::retrieve_key(
        &conn,
        &initiator.user.id,
        &key_id
    ).await? else {
        return Err(error::Error::api(error::GeneralKind::NotFound));
    };

    if let Some(new_key) = json.key {
        if !rfs_lib::sec::authn::totp::recovery::key_valid(&new_key) {
            return Err(error::Error::api((
                error::GeneralKind::InvalidData,
                error::Detail::with_key("key")
            )));
        }

        if totp::recovery::key_exists(&conn, initiator.user().id(), &new_key).await? {
            return Err(error::Error::api(error::GeneralKind::AlreadyExists));
        }

        hash.set_key(new_key);
    }

    if json.regen {
        hash.regen_hash()?;
    }

    let transaction = conn.transaction().await?;

    hash.update(&transaction).await?;

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(rfs_api::auth::totp::TotpRecovery {
        user_id: hash.user_id,
        key: hash.key.into(),
        hash: hash.hash.into(),
        used: hash.used.into(),
    }))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(PathParams { key_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(hash) = totp::recovery::Hash::retrieve_key(
        &conn,
        &initiator.user.id,
        &key_id
    ).await? else {
        return Err(error::Error::api(error::AuthKind::TotpRecoveryNotFound));
    };

    let transaction = conn.transaction().await?;

    hash.delete(&transaction).await?;

    transaction.commit().await?;

    Ok(StatusCode::OK)
}
