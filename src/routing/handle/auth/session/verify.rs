use rfs_api::auth::session::SubmittedVerify;
use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::totp;
use crate::sec::authn::initiator::{self, LookupError};
use crate::sec::authn::session::VerifyMethod;

pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<SubmittedVerify>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let mut session = match initiator::lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(_initiator) => {
            return Err(error::Error::api(error::ApiErrorKind::AlreadyAuthenticated));
        },
        Err(err) => match err {
            LookupError::SessionUnverified(session) => session,
            LookupError::SessionUnauthenticated(_) => {
                return Err(error::Error::api(error::ApiErrorKind::AuthRequired));
            },
            _ => {
                return Err(err.into());
            }
        }
    };

    let transaction = conn.transaction().await?;

    match json {
        SubmittedVerify::Totp(code) => match session.verify_method {
            VerifyMethod::Totp => {
                use rust_otp::VerifyResult;

                let Some(totp) = totp::Totp::retrieve(&transaction, &session.user_id).await? else {
                    return Err(error::Error::new()
                        .source("session required totp verify but user totp was not found"));
                };

                let result = totp.verify(&code)?;

                match result {
                    VerifyResult::Valid => {},
                    _ => {
                        return Err(error::Error::api(error::ApiErrorKind::InvalidTotp));
                    }
                }
            },
            _ => {
                return Err(error::Error::api(error::ApiErrorKind::InvalidAuthMethod));
            }
        },
        SubmittedVerify::TotpHash(hash) => match session.verify_method {
            VerifyMethod::Totp => {
                let Some(mut totp_hash) = totp::recovery::Hash::retrieve_hash(
                    &transaction,
                    &session.user_id,
                    &hash
                ).await? else {
                    return Err(error::Error::api(error::ApiErrorKind::InvalidTotpHash));
                };

                if *totp_hash.used() || !totp_hash.verify(hash) {
                    return Err(error::Error::api(error::ApiErrorKind::InvalidTotpHash));
                }

                totp_hash.set_used();

                totp_hash.update(&transaction).await?;
            },
            _ => {
                return Err(error::Error::api(error::ApiErrorKind::InvalidAuthMethod));
            }
        }
    }

    session.verified = true;

    session.update(&transaction).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
