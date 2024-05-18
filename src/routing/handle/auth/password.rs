use rfs_api::auth::password::CreatePassword;

use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::password::{self, Password};

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<CreatePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    let transaction = conn.transaction().await?;

    if let Some(mut current) = Password::retrieve(&transaction, &initiator.user.id).await? {
        let Some(given) = json.current else {
            return Err(error::Error::api((
                error::ApiErrorKind::MissingData,
                error::Detail::Keys(vec![String::from("current")])
            )));
        };

        if !current.verify(&given, state.sec().peppers())? {
            return Err(error::Error::api((
                error::ApiErrorKind::InvalidData,
                error::Detail::Keys(vec![String::from("password")])
            )));
        }

        current.update(given, state.sec().peppers(), &transaction).await?;
    } else {
        password::Password::create(
            initiator.user.id.clone(),
            json.updated,
            state.sec().peppers(),
            &transaction
        ).await?;
    }

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<rfs_api::auth::password::DeletePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    let transaction = conn.transaction().await?;

    let Some(current) = Password::retrieve(&transaction, &initiator.user.id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::PasswordNotFound));
    };

    if !current.verify(&json.current, state.sec().peppers())? {
        return Err(error::Error::api((
            error::ApiErrorKind::InvalidData,
            error::Detail::Keys(vec![String::from("current")])
        )));
    }

    current.delete(&transaction).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
