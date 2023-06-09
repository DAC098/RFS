use lib::actions::auth::SubmitVerify;
use axum::http::{HeaderMap, StatusCode};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::auth;
use crate::auth::initiator::{self, LookupError};
use crate::auth::session::VerifyMethod;

pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<SubmitVerify>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let mut session = match initiator::lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(initiator) => {
            return Ok(net::Json::empty()
                .with_message("session already verified")
                .into_response());
        },
        Err(err) => match err {
            LookupError::SessionUnverified(session) => session,
            LookupError::SessionUnauthenticated(_) => {
                return Err(error::Error::new()
                    .message("session must be authenticated"));
            },
            _ => {
                return Err(err.into());
            }
        }
    };

    match json {
        SubmitVerify::None => match session.verify_method {
            VerifyMethod::None => {},
            _ => {
                return Err(error::Error::new()
                    .status(StatusCode::UNAUTHORIZED)
                    .kind("InvalidAuthMethod")
                    .message("invalid auth method provided"));
            }
        },
        SubmitVerify::Totp(code) => match session.verify_method {
            VerifyMethod::Totp => {
                use rust_otp::VerifyResult;

                let Some(totp) = auth::totp::Totp::retrieve(&conn, &session.user_id).await? else {
                    return Err(error::Error::new()
                        .source("session required totp verify but user totp was not found"));
                };

                let result = totp.verify(&code)?;

                match result {
                    VerifyResult::Valid => {},
                    _ => {
                        return Err(error::Error::new()
                            .status(StatusCode::UNAUTHORIZED)
                            .kind("InvalidCode")
                            .message("invalid totp code provided"));
                    }
                }
            },
            _ => {
                return Err(error::Error::new()
                    .status(StatusCode::UNAUTHORIZED)
                    .kind("InvalidAuthMethod")
                    .message("invalid auth method provided"));
            }
        },
        SubmitVerify::TotpHash(hash) => match session.verify_method {
            VerifyMethod::Totp => {
                let Some(mut totp_hash) = auth::totp::TotpHash::retrieve_hash(
                    &conn, 
                    &session.user_id, 
                    &hash
                ).await? else {
                    return Err(error::Error::new()
                        .status(StatusCode::UNAUTHORIZED)
                        .kind("TotpHashInvalid")
                        .message("given totp hash is not valid"));
                };

                if *totp_hash.used() || !totp_hash.verify(hash) {
                    return Err(error::Error::new()
                        .status(StatusCode::UNAUTHORIZED)
                        .kind("TotpHashInvalid")
                        .message("given totp hash is not valid"));
                }

                {
                    let transaction = conn.transaction().await?;

                    totp_hash.set_used();

                    totp_hash.update(&transaction).await?;

                    transaction.commit().await?;
                }
            },
            _ => {
                return Err(error::Error::new()
                    .status(StatusCode::UNAUTHORIZED)
                    .kind("InvalidAuthMethod")
                    .message("invalid auth method provided"));
            }
        }
    }

    {
        let transaction = conn.transaction().await?;

        session.verified = true;

        session.update(&transaction).await?;

        transaction.commit().await?;
    }

    Ok(net::Json::empty()
        .with_message("session verified")
        .into_response())
}
