use rfs_api::auth::session::SubmittedAuth;

use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::{totp, password};
use crate::sec::authn::session::{VerifyMethod, AuthMethod};
use crate::sec::authn::initiator::{self, LookupError};
use crate::user;

pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<SubmittedAuth>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let transaction = conn.transaction().await?;

    let mut session = match initiator::lookup_header_map(state.auth(), &transaction, &headers).await {
        Ok(_initiator) => {
            return Err(error::Error::api(error::ApiErrorKind::AlreadyAuthenticated));
        },
        Err(err) => match err {
            LookupError::SessionUnauthenticated(session) => session,
            LookupError::SessionUnverified(_) => {
                return Err(error::Error::api(error::ApiErrorKind::VerifyRequired));
            },
            _ => {
                return Err(err.into());
            }
        }
    };

    match json {
        SubmittedAuth::Password(given) => match session.auth_method {
            AuthMethod::Password => {
                if !rfs_lib::sec::authn::password_valid(&given) {
                    return Err(error::Error::api(error::ApiErrorKind::InvalidPassword));
                }

                let Some(user_password) = password::Password::retrieve(
                    &transaction,
                    &session.user_id
                ).await? else {
                    return Err(error::Error::new()
                        .source("session required user password but user password was not found"));
                };

                if !user_password.verify(&given, state.sec().peppers())? {
                    return Err(error::Error::api(error::ApiErrorKind::InvalidPassword));
                }

                session.authenticated = true;
            },
        }
    }

    match session.verify_method {
        VerifyMethod::None => {
            session.verified = true;

            session.update(&transaction).await?;

            if let Some(user) = user::User::retrieve(&transaction, &session.user_id).await? {
                state.auth()
                    .session_info()
                    .cache()
                    .insert(session.token.clone(), (session, user));
            } else {
                return Err(error::Error::api(
                    error::ApiErrorKind::UserNotFound,
                ));
            }

            transaction.commit().await?;

            Ok(StatusCode::NO_CONTENT.into_response())
        },
        VerifyMethod::Totp => {
            session.update(&transaction).await?;

            let Some(totp) = totp::Totp::retrieve(
                &transaction,
                &session.user_id
            ).await? else {
                return Err(error::Error::new()
                    .source("session required user totp but user totp was not found"));
            };

            if let Some(user) = user::User::retrieve(&transaction, &session.user_id).await? {
                state.auth()
                    .session_info()
                    .cache()
                    .insert(session.token.clone(), (session, user));
            } else {
                return Err(error::Error::api(
                    error::ApiErrorKind::UserNotFound,
                ));
            }

            transaction.commit().await?;

            let verify = rfs_api::auth::session::RequestedVerify::Totp {
                digits: *totp.digits()
            };

            Ok((
                StatusCode::OK,
                rfs_api::Payload::new(verify)
            ).into_response())
        }
    }
}
