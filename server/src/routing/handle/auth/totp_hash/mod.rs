use lib::models::actions;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::auth::initiator::Initiator;
use crate::auth::totp::TotpHash;

pub mod key_id;

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreateTotpHash>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let transaction = conn.transaction().await?;

    TotpHash::builder(initiator.user().id().clone(), json.key)
        .build(&transaction)
        .await?;

    Ok(net::Json::empty()
        .with_status(StatusCode::CREATED)
        .with_message("created totp hash"))
}

