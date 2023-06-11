use lib::actions;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp::Totp;

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreateTotp>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if let Some(existing) = Totp::retrieve(&conn, initiator.user().id()).await? {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("TotpExists")
            .message("totp already exists"));
    }

    let mut new_totp = Totp::builder(initiator.user().id().clone());

    if let Some(algo) = json.algo {
        if !new_totp.set_algo(algo) {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidAlgo")
                .message("algo provided is invalid"));
        }
    }

    if let Some(digits) = json.digits {
        if !new_totp.set_digits(digits) {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidDigits")
                .message("digits provided is invalid"));
        }
    }

    if let Some(step) = json.step {
        if !new_totp.set_step(step) {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidStep")
                .message("step provided is invalid"));
        }
    }

    let transaction = conn.transaction().await?;

    new_totp.build(&transaction).await?;

    transaction.commit().await?;

    Ok(net::Json::empty()
       .with_status(StatusCode::CREATED)
       .with_message("create totp"))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if let Some(totp) = Totp::retrieve(&conn, initiator.user().id()).await? {
        let transaction = conn.transaction().await?;

        totp.delete(&transaction).await?;

        transaction.commit().await?;

        Ok(net::Json::empty()
           .with_message("deleted totp"))
    } else {
        Ok(net::Json::empty()
           .with_message("no totp available"))
    }
}
