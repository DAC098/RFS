use rfs_lib::actions;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::password::Password;

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreatePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if let Some(current) = Password::retrieve(&conn, initiator.user().id()).await? {
        let Some(secret) = state.auth().secrets().get(current.version()) else {
            return Err(error::Error::new()
                .source("password secret version not found. unable to verify user password"));
        };

        let Some(given) = json.current else {
            return Err(error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("PasswordNotProvided")
                .message("current password is required"));
        };

        if !rfs_lib::sec::authn::password_valid(&given) {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidPassword")
                .message("the current password is an invalid format"));
        };

        if !current.verify(given, secret)? {
            return Err(error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("InvalidPassword")
                .message("provided password is invalid"));
        }
    }

    if !rfs_lib::sec::authn::password_valid(&json.updated) {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("InvalidPassword")
            .message("the new password is an invalid format"));
    };

    if json.updated != json.confirm {
        return Err(error::Error::new()
            .status(StatusCode::UNAUTHORIZED)
            .kind("InvalidUpdatedPassword")
            .message("miss match updated and confirmed"));
    }

    let transaction = conn.transaction().await?;

    Password::builder(initiator.user().id().clone())
        .with_secret(state.auth().secrets().latest())
        .build(&transaction)
        .await?;

    transaction.commit().await?;

    Ok(net::Json::empty()
        .with_message("password updated successfully"))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::DeletePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if let Some(current) = Password::retrieve(
        &conn,
        initiator.user().id()
    ).await? {
        let Some(secret) = state.auth().secrets().get(current.version()) else {
            return Err(error::Error::new()
                .source("password secret version not found. unable to verify user password"));
        };

        let Some(given) = json.current else {
            return Err(error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("PasswordNotProvided")
                .message("current password is required"));
        };

        if !current.verify(given, secret)? {
            return Err(error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("InvalidPassword")
                .message("provided password is invalid"));
        }

        let transaction = conn.transaction().await?;

        current.delete(&transaction).await?;

        transaction.commit().await?;

        Ok(net::Json::empty()
           .with_message("password deleted successfuly"))
    } else {
        Ok(net::Json::empty()
           .with_message("no password found"))
    }
}
