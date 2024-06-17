use rfs_api::auth::password::CreatePassword;

use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;

use crate::net::error::{self, Context};
use crate::state::ArcShared;
use crate::sec::authn::initiator::{Initiator, Mechanism};
use crate::sec::authn::password::Password;
use crate::sec::authn::session;

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<CreatePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    let transaction = conn.transaction().await?;

    let mut password = Password::retrieve(&transaction, &initiator.user.id)
        .await?
        .context("missing password for user")?;

    let Some(given) = json.current else {
        return Err(error::Error::api((
            error::ApiErrorKind::MissingData,
            error::Detail::Keys(vec![String::from("current")])
        )));
    };

    if !password.verify(&given, state.sec().peppers())? {
        return Err(error::Error::api((
            error::ApiErrorKind::InvalidData,
            error::Detail::Keys(vec![String::from("password")])
        )));
    }

    password.update(&transaction, given, state.sec().peppers()).await?;

    match initiator.mechanism {
        Mechanism::Session(session) => {
            let cache = state.auth().session_info().cache();
            let session_tokens = session::Session::delete_user_sessions(
                &transaction,
                &initiator.user.id,
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
