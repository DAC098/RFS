use std::convert::Into;

use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{Context, ApiErrorKind};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp;

#[derive(Deserialize)]
pub struct RecoveryKeyPath {
    key_id: String
}

pub async fn retrieve(
    State(state): State<ArcShared>,
    initiator: Initiator,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    let totp = totp::Totp::retrieve(&conn, &initiator.user.id)
        .await?
        .kind(ApiErrorKind::TotpNotFound)?;

    Ok(rfs_api::Payload::new(rfs_api::users::totp::Totp {
        algo: totp.algo.get().to_string(),
        secret: totp.secret.into(),
        digits: totp.digits.into(),
        step: totp.step.into()
    }))
}

pub async fn create(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<rfs_api::users::totp::CreateTotp>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    if let Some(_existing) = totp::Totp::retrieve(&conn, &initiator.user.id).await? {
        return Err(ApiError::from(ApiErrorKind::AlreadyExists));
    }

    let algo = json.algo.unwrap_or(totp::Algo::SHA512);
    let digits = json.digits.unwrap_or(8);
    let step = json.step.unwrap_or(30);

    let secret = totp::create_secret()?;

    let transaction = conn.transaction().await?;

    {
        let pg_algo = algo.as_i16();

        transaction.execute(
            "\
            insert into auth_totp (user_id, algo, secret, digits, step) values \
            ($1, $2, $3, $4, $5)",
            &[
                &initiator.user.id,
                &pg_algo,
                &secret.as_slice(),
                &(digits as i32),
                &(step as i32)
            ]
        ).await?;
    }

    transaction.commit().await?;

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(rfs_api::users::totp::Totp {
            algo: algo.to_string(),
            secret,
            digits,
            step
        })
    ))
}

pub async fn update(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<rfs_api::users::totp::UpdateTotp>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let mut regen = false;

    json.validate()?;

    let mut totp = totp::Totp::retrieve(&conn, &initiator.user.id)
        .await?
        .kind(ApiErrorKind::TotpNotFound)?;

    if let Some(given) = json.algo {
        totp.set_algo(given);
        regen = true;
    }

    if let Some(given) = json.digits {
        totp.set_digits(given);
        regen = true;
    }

    if let Some(given) = json.step {
        totp.set_step(given);
        regen = true;
    }

    if regen || json.regen {
        totp.regen_secret()?;
    }

    let transaction = conn.transaction().await?;

    totp.update(&transaction).await?;

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(rfs_api::users::totp::Totp {
        algo: totp.algo.get().to_string(),
        secret: totp.secret.into(),
        digits: totp.digits.into(),
        step: totp.step.into()
    }))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let record = totp::Totp::retrieve(&conn, &initiator.user.id)
        .await?
        .kind(ApiErrorKind::TotpNotFound)?;

    let transaction = conn.transaction().await?;

    record.delete(&transaction).await?;

    transaction.execute(
        "delete from auth_totp_hash where user_id = $1",
        &[&initiator.user.id]
    ).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn retrieve_recovery(
    State(state): State<ArcShared>,
    initiator: Initiator,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    let result = conn.query_raw(
        "\
        select auth_totp_hash.user_id, \
               auth_totp_hash.key, \
               auth_totp_hash.hash, \
               auth_totp_hash.used \
        from auth_totp_hash \
        where auth_totp_hash.user_id = $1",
        &[&initiator.user.id]
    ).await?;

    futures::pin_mut!(result);

    let mut rtn = Vec::new();

    while let Some(row) = result.try_next().await? {
        rtn.push(rfs_api::users::totp::TotpRecovery {
            user_id: row.get(0),
            key: row.get(1),
            hash: row.get(2),
            used: row.get(3),
        });
    }

    Ok(rfs_api::Payload::new(rtn))
}

pub async fn create_recovery(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<rfs_api::users::totp::CreateTotpHash>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    if totp::recovery::key_exists(&conn, &initiator.user.id, &json.key).await? {
        return Err(ApiError::from(ApiErrorKind::AlreadyExists));
    }

    let hash = totp::recovery::create_hash()?;

    let transaction = conn.transaction().await?;

    transaction.execute(
        "\
        insert into auth_totp_hash (user_id, key, hash, used) values \
        ($1, $2, $3, false)",
        &[&initiator.user.id, &json.key, &hash]
    ).await?;

    transaction.commit().await?;

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(rfs_api::users::totp::TotpRecovery {
            user_id: initiator.user.id.clone(),
            key: json.key,
            hash,
            used: false
        })
    ))
}

pub async fn retrieve_recovery_key(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(RecoveryKeyPath { key_id }): Path<RecoveryKeyPath>
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    let hash = totp::recovery::Hash::retrieve_key(&conn, &initiator.user.id, &key_id)
        .await?
        .kind(ApiErrorKind::TotpRecoveryNotFound)?;

    Ok(rfs_api::Payload::new(rfs_api::users::totp::TotpRecovery {
        user_id: hash.user_id,
        key: hash.key.into(),
        hash: hash.hash.into(),
        used: hash.used.into(),
    }))
}

pub async fn update_recovery_key(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(RecoveryKeyPath { key_id }): Path<RecoveryKeyPath>,
    axum::Json(json): axum::Json<rfs_api::users::totp::UpdateTotpHash>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    let mut hash = totp::recovery::Hash::retrieve_key(&conn, &initiator.user.id, &key_id)
        .await?
        .kind(ApiErrorKind::NotFound)?;

    if let Some(new_key) = json.key {
        if totp::recovery::key_exists(&conn, &initiator.user.id, &new_key).await? {
            return Err(ApiError::from(ApiErrorKind::AlreadyExists));
        }

        hash.set_key(new_key);
    }

    if json.regen {
        hash.regen_hash()?;
    }

    let transaction = conn.transaction().await?;

    hash.update(&transaction).await?;

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(rfs_api::users::totp::TotpRecovery {
        user_id: hash.user_id,
        key: hash.key.into(),
        hash: hash.hash.into(),
        used: hash.used.into(),
    }))
}

pub async fn delete_recovery_key(
    State(state): State<ArcShared>,
    initiator: Initiator,
    Path(RecoveryKeyPath { key_id }): Path<RecoveryKeyPath>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let hash = totp::recovery::Hash::retrieve_key(&conn, &initiator.user.id, &key_id)
        .await?
        .kind(ApiErrorKind::TotpRecoveryNotFound)?;

    let transaction = conn.transaction().await?;

    hash.delete(&transaction).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
