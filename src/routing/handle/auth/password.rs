use rfs_api::auth::password::CreatePassword;

use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error::{self, Context};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::password::Password;

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<CreatePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    let transaction = conn.transaction().await?;

    let mut current = Password::retrieve(&transaction, &initiator.user.id)
        .await?
        .context("missing password for user")?;

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

    current.update(&transaction, given, state.sec().peppers()).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
