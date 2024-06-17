use axum::debug_handler;
use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error::{self, Context};
use crate::state::ArcShared;
use crate::user;
use crate::sec::authn::{session, Authenticate, Verify};
use crate::sec::authn::initiator::{self, LookupError};

#[debug_handler]
pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<rfs_api::auth::session::RequestUser>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    match initiator::lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(_) => {
            return Err(error::Error::api(
                error::ApiErrorKind::AlreadyAuthenticated
            ));
        },
        Err(err) => match err {
            LookupError::MechanismNotFound => {},
            _ => {
                return Err(err.into());
            }
        }
    }

    let Some(user) = user::User::query_with_username(&mut conn, &json.username).await? else {
        return Err(error::Error::api(
            error::ApiErrorKind::UserNotFound,
        ));
    };

    let mut builder = session::Session::builder(user.id().clone());
    let transaction = conn.transaction().await?;

    let auth_method = Authenticate::retrieve_primary(&transaction, user.id())
        .await?
        .context("missing authentication method for user")?;

    let payload = match auth_method {
        Authenticate::Password(_) => {
            builder.auth_method(session::AuthMethod::Password);

            rfs_api::Payload::new(rfs_api::auth::session::RequestedAuth::Password)
        }
    };

    if let Some(verify_method) = Verify::retrieve_primary(&transaction, user.id()).await? {
        match verify_method {
            Verify::Totp(_) => {
                builder.verify_method(session::VerifyMethod::Totp);
            }
        }
    }

    let session = builder.build(&transaction).await?;

    transaction.commit().await?;

    let session_cookie = session::create_session_cookie(state.auth(), &session)
        .ok_or(error::Error::new().source("session keys rwlock poisoned"))?;

    state.auth()
        .session_info()
        .cache()
        .insert(session.token.clone(), (session, user));

    Ok((
        StatusCode::OK,
        session_cookie,
        payload,
    ))
}
