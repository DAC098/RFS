use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::{totp, password};
use crate::sec::authn::session::{VerifyMethod, AuthMethod};
use crate::sec::authn::initiator::{self, LookupError};

pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<rfs_api::auth::session::SubmittedAuth>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let peppers = state.sec().peppers().inner();

    let mut session = match initiator::lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(_initiator) => {
            return Err(error::Error::api(error::AuthKind::AlreadyAuthenticated));
        },
        Err(err) => match err {
            LookupError::SessionUnauthenticated(session) => session,
            LookupError::SessionUnverified(_) => {
                return Err(error::Error::api(error::AuthKind::VerifyRequired));
            },
            _ => {
                return Err(err.into());
            }
        }
    };

    let transaction = conn.transaction().await?;

    match json {
        rfs_api::auth::session::SubmittedAuth::Password(given) => match session.auth_method {
            AuthMethod::Password => {
                if !rfs_lib::sec::authn::password_valid(&given) {
                    return Err(error::Error::api(error::AuthKind::InvalidPassword));
                };

                let Some(user_password) = password::Password::retrieve(
                    &transaction,
                    &session.user_id
                ).await? else {
                    return Err(error::Error::new()
                        .source("session required user password but user password was not found"));
                };

                {
                    let Ok(reader) = peppers.read() else {
                        return Err(error::Error::new().source("peppers rwlock poisoned"));
                    };

                    let Some(pepper) = reader.get(&user_password.version) else {
                        return Err(error::Error::new()
                            .source("password secret version not found. unable verify user password"));
                    };

                    if !user_password.verify(&given, pepper.data())? {
                        return Err(error::Error::api(error::AuthKind::InvalidPassword));
                    }
                }

                session.authenticated = true;
            },
            _ => {
                return Err(error::Error::api(error::AuthKind::InvalidAuthMethod));
            }
        }
    }

    match session.verify_method {
        VerifyMethod::None => {
            session.verified = true;

            session.update(&transaction).await?;

            transaction.commit().await?;

            Ok(StatusCode::NO_CONTENT.into_response())
        },
        VerifyMethod::Totp => {
            session.update(&transaction).await?;

            transaction.commit().await?;

            let Some(totp) = totp::Totp::retrieve(
                &conn,
                &session.user_id
            ).await? else {
                return Err(error::Error::new()
                    .source("session required user totp but user totp was not found"));
            };

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
