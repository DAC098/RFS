use rfs_api::auth::session::SubmittedAuth;
use rfs_api::auth::session::SubmittedVerify;

use axum::debug_handler;
use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{Context, ApiErrorKind};
use crate::state::ArcShared;
use crate::user;
use crate::sec::authn::{totp, password, Authenticate, Verify};
use crate::sec::authn::session::{self, VerifyMethod, AuthMethod};
use crate::sec::authn::initiator::{self, Mechanism, LookupError};

#[debug_handler]
pub async fn request(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<rfs_api::auth::session::RequestUser>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    match initiator::lookup_header_map(state.sec(), &conn, &headers).await {
        Ok(_) => {
            return Err(ApiError::from(ApiErrorKind::AlreadyAuthenticated));
        },
        Err(err) => match err {
            LookupError::MechanismNotFound => {},
            _ => {
                return Err(err.into());
            }
        }
    }

    let user = user::User::query_with_username(&mut conn, &json.username)
        .await?
        .kind(ApiErrorKind::UserNotFound)?;

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

    let session_cookie = session::create_session_cookie(state.sec(), &session)
        .context("session keys rwlock poisoned")?;

    state.sec()
        .session_info()
        .cache()
        .insert(session.token.clone(), (session, user));

    Ok((
        StatusCode::OK,
        session_cookie,
        payload,
    ))
}


pub async fn submit(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<SubmittedAuth>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    json.validate()?;

    let transaction = conn.transaction().await?;

    let mut session = match initiator::lookup_header_map(state.sec(), &transaction, &headers).await {
        Ok(_initiator) => {
            return Err(ApiError::from(ApiErrorKind::AlreadyAuthenticated));
        },
        Err(err) => match err {
            LookupError::SessionUnauthenticated(session) => session,
            LookupError::SessionUnverified(_) => {
                return Err(ApiError::from(ApiErrorKind::VerifyRequired));
            },
            _ => {
                return Err(err.into());
            }
        }
    };

    match json {
        SubmittedAuth::Password(given) => match session.auth_method {
            AuthMethod::Password => {
                let user_password = password::Password::retrieve(&transaction, &session.user_id)
                    .await?
                    .context("session required user password but user password was not found")?;

                if !user_password.verify(&given, state.sec().peppers())? {
                    return Err(ApiError::from(ApiErrorKind::InvalidPassword));
                }

                session.authenticated = true;
            },
        }
    }

    match session.verify_method {
        VerifyMethod::None => {
            session.verified = true;

            session.update(&transaction).await?;

            let user = user::User::retrieve(&transaction, &session.user_id)
                .await?
                .kind(ApiErrorKind::UserNotFound)?;

            state.sec()
                .session_info()
                .cache()
                .insert(session.token.clone(), (session, user));

            transaction.commit().await?;

            Ok(StatusCode::NO_CONTENT.into_response())
        },
        VerifyMethod::Totp => {
            session.update(&transaction).await?;

            let totp = totp::Totp::retrieve(&transaction, &session.user_id)
                .await?
                .context("session required user totp but user totp was not found")?;
            let user = user::User::retrieve(&transaction, &session.user_id)
                .await?
                .kind(ApiErrorKind::UserNotFound)?;

            state.sec()
                .session_info()
                .cache()
                .insert(session.token.clone(), (session, user));

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

pub async fn verify(
    State(state): State<ArcShared>,
    headers: HeaderMap,
    axum::Json(json): axum::Json<SubmittedVerify>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let mut session = match initiator::lookup_header_map(state.sec(), &conn, &headers).await {
        Ok(_initiator) => {
            return Err(ApiError::from(ApiErrorKind::AlreadyAuthenticated));
        },
        Err(err) => match err {
            LookupError::SessionUnverified(session) => session,
            LookupError::SessionUnauthenticated(_) => {
                return Err(ApiError::from(ApiErrorKind::AuthRequired));
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

                let totp = totp::Totp::retrieve(&transaction, &session.user_id)
                    .await?
                    .context("session required totp verify but user totp was not found")?;

                let result = totp.verify(&code)?;

                match result {
                    VerifyResult::Valid => {},
                    _ => {
                        return Err(ApiError::from(ApiErrorKind::InvalidTotp));
                    }
                }
            },
            _ => {
                return Err(ApiError::from(ApiErrorKind::InvalidAuthMethod));
            }
        },
        SubmittedVerify::TotpHash(hash) => match session.verify_method {
            VerifyMethod::Totp => {
                let mut totp_hash = totp::recovery::Hash::retrieve_hash(
                    &transaction,
                    &session.user_id,
                    &hash
                ).await?
                    .kind(ApiErrorKind::InvalidTotpHash)?;

                if *totp_hash.used() || !totp_hash.verify(hash) {
                    return Err(ApiError::api(ApiErrorKind::InvalidTotpHash));
                }

                totp_hash.set_used();

                totp_hash.update(&transaction).await?;
            },
            _ => {
                return Err(ApiError::from(ApiErrorKind::InvalidAuthMethod));
            }
        }
    }

    session.verified = true;

    session.update(&transaction).await?;

    let user = user::User::retrieve(&transaction, &session.user_id)
        .await?
        .kind(ApiErrorKind::UserNotFound)?;

    state.sec()
        .session_info()
        .cache()
        .insert(session.token.clone(), (session, user));

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn drop(
    State(state): State<ArcShared>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let session = match initiator::lookup_header_map(state.sec(), &conn, &headers).await {
        Ok(initiator) => match initiator.mechanism {
            Mechanism::Session(session) => session,
        }
        Err(err) => match err {
            LookupError::SessionNotFound => {
                return Ok((
                    StatusCode::NO_CONTENT,
                    session::expire_session_cookie(state.sec())
                ));
            }
            LookupError::SessionExpired(session) |
            LookupError::SessionUnauthenticated(session) |
            LookupError::SessionUnverified(session) => session,
            LookupError::UserNotFound(mechanism) => match mechanism {
                Mechanism::Session(session) => session
            }
            err => {
                return Err(err.into());
            }
        }
    };

    let transaction = conn.transaction().await?;

    session.delete(&transaction).await?;

    transaction.commit().await?;

    state.sec()
        .session_info()
        .cache()
        .invalidate(&session.token);

    Ok((
        StatusCode::NO_CONTENT,
        session::expire_session_cookie(state.sec())
    ))
}
