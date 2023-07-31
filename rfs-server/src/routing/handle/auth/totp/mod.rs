use rfs_lib::actions;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use rand::RngCore;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp;

pub mod recovery;

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreateTotp>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if let Some(_existing) = totp::Totp::retrieve(&conn, initiator.user().id()).await? {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("TotpExists")
            .message("totp already exists"));
    }

    let algo = if let Some(given) = json.algo {
        let Ok(algo) = totp::Algo::try_from(given) else {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidAlgo")
                .message("algo provided is invalid"));
        };

        algo
    } else {
        totp::Algo::SHA512
    };

    let digits: u32 = if let Some(given) = json.digits {
        if given > 12 {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidDigits")
                .message("digits provided is invalid"));
        }

        given
    } else {
        8
    };

    let step: u64 = if let Some(given) = json.step {
        if given > 120 {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidStep")
                .message("step provided is invalid"));
        }

        given
    } else {
        30
    };

    let mut secret = [0u8; totp::SECRET_LEN];
    rand::thread_rng().try_fill_bytes(&mut secret)?;

    let transaction = conn.transaction().await?;

    {
        let pg_algo = algo.as_i16();

        transaction.execute(
            "\
            insert into auth_totp (user_id, algo, secret, digits, step) values \
            ($1, $2, $3, $4, $5)",
            &[
                initiator.user().id(),
                &pg_algo,
                &secret.as_slice(),
                &(digits as i32),
                &(step as i64)
            ]
        ).await?;
    }

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

    if let Some(totp) = totp::Totp::retrieve(&conn, initiator.user().id()).await? {
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
