use std::convert::Into;

use rfs_lib::actions;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp;

pub mod recovery;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: Initiator,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    let Some(totp) = totp::Totp::retrieve(&conn, initiator.user().id()).await? else {
        return Err(error::Error::api(error::GeneralKind::NotFound));
    };

    let rtn = rfs_lib::json::Wrapper::new(rfs_lib::schema::auth::Totp {
        algo: totp.algo.get().to_string(),
        secret: totp.secret.into(),
        digits: totp.digits.into(),
        step: totp.step.into()
    });

    Ok(net::Json::new(rtn))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreateTotp>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if let Some(_existing) = totp::Totp::retrieve(&conn, initiator.user().id()).await? {
        return Err(error::Error::api(error::GeneralKind::AlreadyExists));
    }

    let algo = if let Some(given) = json.algo {
        let Ok(algo) = totp::Algo::try_from(given) else {
            return Err(error::Error::api((
                error::GeneralKind::InvalidData,
                error::Detail::with_key("algo")
            )));
        };

        algo
    } else {
        totp::Algo::SHA512
    };

    let digits: u32 = if let Some(given) = json.digits {
        if !rfs_lib::sec::authn::totp::digits_valid(&given) {
            return Err(error::Error::api((
                error::GeneralKind::InvalidData,
                error::Detail::with_key("digits")
            )));
        }

        given
    } else {
        8
    };

    let step: u64 = if let Some(given) = json.step {
        if !rfs_lib::sec::authn::totp::step_valid(&given) {
            return Err(error::Error::api((
                error::GeneralKind::InvalidData,
                error::Detail::with_key("step")
            )));
        }

        given
    } else {
        30
    };

    let secret = totp::create_secret()?;

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
                &(step as i32)
            ]
        ).await?;
    }

    transaction.commit().await?;

    let rtn = rfs_lib::json::Wrapper::new(rfs_lib::schema::auth::Totp {
        algo: algo.to_string(),
        secret,
        digits,
        step
    })
        .with_message("created totp");

    Ok(net::Json::new(rtn)
       .with_status(StatusCode::CREATED))
}

pub async fn patch(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::UpdateTotp>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let mut regen = false;

    let Some(mut totp) = totp::Totp::retrieve(&conn, initiator.user().id()).await? else {
        return Err(error::Error::api(error::GeneralKind::NotFound));
    };

    if let Some(given) = json.algo {
        let Ok(algo) = totp::Algo::try_from(given) else {
            return Err(error::Error::api((
                error::GeneralKind::InvalidData,
                error::Detail::with_key("algo")
            )));
        };

        totp.set_algo(algo);
        regen = true;
    }

    if let Some(given) = json.digits {
        if !rfs_lib::sec::authn::totp::digits_valid(&given) {
            return Err(error::Error::api((
                error::GeneralKind::InvalidData,
                error::Detail::with_key("digits")
            )));
        }

        totp.set_digits(given);
        regen = true;
    }

    if let Some(given) = json.step {
        if !rfs_lib::sec::authn::totp::step_valid(&given) {
            return Err(error::Error::api((
                error::GeneralKind::InvalidData,
                error::Detail::with_key("step")
            )));
        }

        totp.set_step(given);
        regen = true;
    }

    if regen || json.regen {
        totp.regen_secret()?;
    }

    let transaction = conn.transaction().await?;

    totp.update(&transaction).await?;

    transaction.commit().await?;

    let rtn = rfs_lib::json::Wrapper::new(rfs_lib::schema::auth::Totp {
        algo: totp.algo.get().to_string(),
        secret: totp.secret.into(),
        digits: totp.digits.into(),
        step: totp.step.into()
    });

    Ok(net::Json::new(rtn))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if let Some(totp) = totp::Totp::retrieve(&conn, initiator.user().id()).await? {
        let transaction = conn.transaction().await?;

        totp.delete(&transaction).await?;

        transaction.execute(
            "delete from auth_totp_hash where user_id = $1",
            &[initiator.user().id()]
        ).await?;

        transaction.commit().await?;

        Ok(net::Json::empty()
           .with_message("deleted totp"))
    } else {
        Ok(net::Json::empty()
           .with_message("no totp available"))
    }
}
