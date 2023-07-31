use rfs_lib::actions;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp;

pub mod key_id;

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreateTotpHash>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !rfs_lib::sec::authn::totp::recovery::key_valid(&json.key) {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("InvalidKey")
            .message("the provided key is an invalid format"));
    };

    if totp::recovery::key_exists(&conn, initiator.user().id(), &json.key).await? {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("KeyExits")
            .message("the provided key already exists"));
    }

    let hash = totp::recovery::create_hash()?;

    let transaction = conn.transaction().await?;

    transaction.execute(
        "\
        insert into auth_totp_hash (user_id, key, hash, used) values \
        ($1 $2, $3, false)",
        &[initiator.user().id(), &json.key, &hash]
    ).await?;

    transaction.commit().await?;

    Ok(net::Json::empty()
        .with_status(StatusCode::CREATED)
        .with_message("created totp hash"))
}

