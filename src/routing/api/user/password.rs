use rfs_api::users::password::CreatePassword;

use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;

use crate::error::{ApiResult, ApiError};
use crate::error::api::{Context, Detail, ApiErrorKind};
use crate::state::ArcShared;
use crate::sec::authn::initiator::{Initiator, Mechanism};
use crate::sec::authn::password::Password;
use crate::sec::authn::session;

pub async fn update(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<CreatePassword>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    let transaction = conn.transaction().await?;

    let mut password = Password::retrieve(&transaction, initiator.user.id.local())
        .await?
        .context("missing password for user")?;

    if !password.verify(&json.current, state.sec().peppers())? {
        return Err(ApiError::from((
            ApiErrorKind::InvalidData,
            Detail::Keys(vec![String::from("password")])
        )));
    }

    password.update(&transaction, json.updated, state.sec().peppers()).await?;

    match initiator.mechanism {
        Mechanism::Session(session) => {
            let cache = state.sec().session_info().cache();
            let session_tokens = session::Session::delete_user_sessions(
                &transaction,
                initiator.user.id.local(),
                Some(&session.token)
            ).await?;

            futures::pin_mut!(session_tokens);

            while let Some(token) = session_tokens.try_next().await? {
                cache.invalidate(&token);
            }
        }
    }

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
