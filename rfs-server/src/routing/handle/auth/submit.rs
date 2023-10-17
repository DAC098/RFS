use rfs_lib::schema;
use rfs_lib::actions;
use axum::http::{HeaderMap, StatusCode};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::{totp, password};
use crate::sec::authn::session::{VerifyMethod, AuthMethod};
use crate::sec::authn::initiator::{self, LookupError};

pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<actions::auth::SubmitAuth>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let peppers = state.sec().peppers().inner();

    let mut session = match initiator::lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(_initiator) => {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("AlreadyAuthenticated")
                .message("session already authenticated"));
        },
        Err(err) => match err {
            LookupError::SessionUnauthenticated(session) => session,
            LookupError::SessionUnverified(_) => {
                return Err(error::Error::new()
                    .status(StatusCode::BAD_REQUEST)
                    .kind("VerifyRequired")
                    .message("session already authenticated, must verify"));
            },
            _ => {
                return Err(err.into());
            }
        }
    };

    match json {
        actions::auth::SubmitAuth::None => match session.auth_method {
            AuthMethod::None => {
                session.authenticated = true;
            },
            _ => {
                return Err(error::Error::new()
                    .status(StatusCode::UNAUTHORIZED)
                    .kind("InvalidAuthMethod")
                    .message("invalid auth method provided"));
            }
        },
        actions::auth::SubmitAuth::Password(given) => match session.auth_method {
            AuthMethod::Password => {
                if !rfs_lib::sec::authn::password_valid(&given) {
                    return Err(error::Error::new()
                        .status(StatusCode::BAD_REQUEST)
                        .kind("InvalidPassword")
                        .message("the provided password is an invalid format"));
                };

                let Some(user_password) = password::Password::retrieve(
                    &conn,
                    &session.user_id
                ).await? else {
                    return Err(error::Error::new()
                        .source("session required user password but user password was not found"));
                };

                {
                    let reader = peppers.read()
                        .map_err(|_| error::Error::new().source("peppers rwlock poisoned"))?;

                    let Some(pepper) = reader.get(&user_password.version) else {
                        return Err(error::Error::new()
                            .source("password secret version not found. unable verify user password"));
                    };

                    if !user_password.verify(&given, pepper.data())? {
                        return Err(error::Error::new()
                            .status(StatusCode::UNAUTHORIZED)
                            .kind("InvalidPassword")
                            .message("provided password is invalid"));
                    }
                }

                session.authenticated = true;
            },
            _ => {
                return Err(error::Error::new()
                    .status(StatusCode::UNAUTHORIZED)
                    .kind("InvalidAuthMethod")
                    .message("invalid auth method provided"));
            }
        }
    }

    let verify = match session.verify_method {
        VerifyMethod::None => {
            session.verified = true;

            schema::auth::VerifyMethod::None
        },
        VerifyMethod::Totp => {
            let Some(totp) = totp::Totp::retrieve(
                &conn,
                &session.user_id
            ).await? else {
                return Err(error::Error::new()
                    .source("session required user totp but user totp was not found"));
            };

            schema::auth::VerifyMethod::Totp {
                digits: *totp.digits()
            }
        }
    };

    {
        let transaction = conn.transaction().await?;

        session.update(&transaction).await?;

        transaction.commit().await?;
    }

    let json_root = rfs_lib::json::Wrapper::new(verify);

    Ok(net::Json::new(json_root).into_response())
}
