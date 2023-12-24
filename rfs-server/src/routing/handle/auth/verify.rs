use rfs_lib::actions::auth::SubmitVerify;
use axum::http::{HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::totp;
use crate::sec::authn::initiator::{self, LookupError};
use crate::sec::authn::session::VerifyMethod;

pub async fn post(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<SubmitVerify>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let mut session = match initiator::lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(_initiator) => {
            return Err(error::Error::api(error::AuthKind::AlreadyAuthenticated));
        },
        Err(err) => match err {
            LookupError::SessionUnverified(session) => session,
            LookupError::SessionUnauthenticated(_) => {
                return Err(error::Error::api(error::AuthKind::AuthRequired));
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
                return Err(error::Error::api(error::AuthKind::InvalidAuthMethod));
            }
        },
        SubmitVerify::Totp(code) => match session.verify_method {
            VerifyMethod::Totp => {
                use rust_otp::VerifyResult;

                let Some(totp) = totp::Totp::retrieve(&conn, &session.user_id).await? else {
                    return Err(error::Error::new()
                        .source("session required totp verify but user totp was not found"));
                };

                let result = totp.verify(&code)?;

                match result {
                    VerifyResult::Valid => {},
                    _ => {
                        return Err(error::Error::api(error::AuthKind::InvalidTotp,));
                    }
                }
            },
            _ => {
                return Err(error::Error::api(error::AuthKind::InvalidAuthMethod));
            }
        },
        SubmitVerify::TotpHash(hash) => match session.verify_method {
            VerifyMethod::Totp => {
                let Some(mut totp_hash) = totp::recovery::Hash::retrieve_hash(
                    &conn, 
                    &session.user_id, 
                    &hash
                ).await? else {
                    return Err(error::Error::api(error::AuthKind::InvalidTotpHash));
                };

                if *totp_hash.used() || !totp_hash.verify(hash) {
                    return Err(error::Error::api(error::AuthKind::InvalidTotpHash));
                }

                totp_hash.set_used();

                {
                    let transaction = conn.transaction().await?;

                    totp_hash.update(&transaction).await?;

                    transaction.commit().await?;
                }
            },
            _ => {
                return Err(error::Error::api(error::AuthKind::InvalidAuthMethod));
            }
        }
    }

    session.verified = true;

    {
        let transaction = conn.transaction().await?;

        session.update(&transaction).await?;

        transaction.commit().await?;
    }

    Ok(net::Json::empty()
        .with_message("session verified")
        .into_response())
}
