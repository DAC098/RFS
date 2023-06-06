use lib::models;
use axum::http::{HeaderMap, StatusCode};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::user;
use crate::auth;
use crate::auth::session;

pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<models::actions::auth::RequestAuth>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let Some(user) = user::User::query_with_username(&mut conn, &json.username).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("UserNotFound")
            .message("provided username was not found"));
    };

    let duration = chrono::Duration::days(7);
    let issued_on = chrono::Utc::now();
    let expires = issued_on.clone()
        .checked_add_signed(duration)
        .ok_or(error::Error::new()
            .source("chrono::DateTime<Utc> overflowed when adding 7 days"))?;

    let mut json_auth_method = models::actions::auth::AuthMethod::None;
    let mut session = auth::session::Session {
        token: session::token::SessionToken::unique(&conn).await?.unwrap(),
        user_id: user.id().clone(),
        dropped: false,
        issued_on,
        expires,
        authenticated: false,
        verified: false,
        auth_method: session::AuthMethod::None,
        verify_method: session::VerifyMethod::None,
    };

    if let Some(auth_method) = auth::Authorize::retrieve_primary(&conn, user.id()).await? {
        match auth_method {
            auth::Authorize::Password(_) => {
                session.auth_method = session::AuthMethod::Password;
                json_auth_method = models::actions::auth::AuthMethod::Password;
            }
        }

        if let Some(verify_method) = auth::Verify::retrieve_primary(&conn, user.id()).await? {
            match verify_method {
                auth::Verify::Totp(_) => {
                    session.verify_method = session::VerifyMethod::Totp;
                }
            }
        }
    }

    {
        let transaction = conn.transaction().await?;

        session.create(&transaction).await?;

        transaction.commit().await?;
    }

    let session_cookie = session::create_session_cookie(state.auth(), &session);
    let json_root = net::JsonWrapper::new(json_auth_method)
        .with_message("proceed with requested auth method");

    Ok(net::Json::new(json_root)
        .with_header("set-cookie", session_cookie))
}
