use lib::models;
use lib::actions;
use axum::debug_handler;
use axum::http::{HeaderMap, StatusCode};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::user;
use crate::sec::authn::{session, Authenticate, Verify};
use crate::sec::authn::initiator::{self, LookupError};

#[debug_handler]
pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<actions::auth::RequestUser>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    match initiator::lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(_) => {
            return Ok(net::Json::empty()
                .with_message("session already authenticated")
                .into_response());
        },
        Err(err) => match err {
            LookupError::MechanismNotFound => {},
            _ => {
                return Err(err.into());
            }
        }
    }

    let Some(user) = user::User::query_with_username(&mut conn, &json.username).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("UserNotFound")
            .message("provided username was not found"));
    };

    let mut builder = session::Session::builder(user.id().clone());

    let mut json_auth_method = models::auth::AuthMethod::None;
    let mut json_message = String::from("session authenticated");

    if let Some(auth_method) = Authenticate::retrieve_primary(&conn, user.id()).await? {
        match auth_method {
            Authenticate::Password(_) => {
                builder.auth_method(session::AuthMethod::Password);

                json_auth_method = models::auth::AuthMethod::Password;
                json_message = String::from("proceed with requested auth method");
            }
        }

        if let Some(verify_method) = Verify::retrieve_primary(&conn, user.id()).await? {
            match verify_method {
                Verify::Totp(_) => {
                    builder.verify_method(session::VerifyMethod::Totp);
                }
            }
        }
    }

    let session;

    {
        let transaction = conn.transaction().await?;

        session = builder.build(&transaction).await?;

        transaction.commit().await?;
    }

    let session_cookie = session::create_session_cookie(state.auth(), &session);
    let json_root = lib::json::Wrapper::new(json_auth_method)
        .with_message(json_message);

    Ok(net::Json::new(json_root)
        .with_header("set-cookie", session_cookie)
        .into_response())
}
