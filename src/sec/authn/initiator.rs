use std::pin::Pin;
use std::future::Future;

use axum::http::header::{HeaderMap, HeaderValue, GetAll};
use axum::http::request::Parts;
use axum::extract::FromRequestParts;
use deadpool_postgres::GenericClient;

use crate::error::ApiError;
use crate::error::api::ApiErrorKind;
use crate::sec::state;
use crate::user;
use crate::state::ArcShared;

use super::session;

// not sure what to call this
#[derive(Debug)]
pub enum Mechanism {
    Session(session::Session),
}

pub struct Initiator {
    pub user: user::User,
    pub mechanism: Mechanism
}

#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    #[error("session was not found")]
    SessionNotFound,

    #[error("session has expired")]
    SessionExpired(session::Session),

    #[error("session is unauthenticated")]
    SessionUnauthenticated(session::Session),

    #[error("session is unverified")]
    SessionUnverified(session::Session),

    #[error("user was not found")]
    UserNotFound(Mechanism),

    #[error("no authentication mechanism was found")]
    MechanismNotFound,

    #[error(transparent)]
    SessionDecode(#[from] session::DecodeError),

    #[error(transparent)]
    Database(#[from] tokio_postgres::Error),

    #[error(transparent)]
    HeaderToStr(#[from] axum::http::header::ToStrError),
}

impl From<LookupError> for ApiError {
    fn from(e: LookupError) -> Self {
        match e {
            LookupError::SessionNotFound => ApiError::from(ApiErrorKind::SessionNotFound),
            LookupError::SessionExpired(_session) => ApiError::from(ApiErrorKind::SessionExpired),
            LookupError::SessionUnauthenticated(_session) => ApiError::from(ApiErrorKind::SessionUnauthenticated),
            LookupError::SessionUnverified(_session) => ApiError::from(ApiErrorKind::SessionUnverified),

            LookupError::UserNotFound(_authorization) => ApiError::from(ApiErrorKind::UserNotFound),

            LookupError::MechanismNotFound => ApiError::from(ApiErrorKind::MechanismNotFound),

            LookupError::Database(e) => e.into(),
            LookupError::HeaderToStr(e) => e.into(),

            LookupError::SessionDecode(err) => match err {
                session::DecodeError::InvalidString |
                session::DecodeError::InvalidLength |
                session::DecodeError::InvalidHash => ApiError::from(ApiErrorKind::InvalidSession),
                err => ApiError::new().source(err)
            }
        }
    }
}

pub async fn lookup_session_id<S>(
    auth: &state::Sec,
    conn: &impl GenericClient,
    session_id: S
) -> Result<Initiator, LookupError>
where
    S: AsRef<[u8]>
{
    let now = chrono::Utc::now();
    let (token, _hash) = session::decode_base64(auth, session_id)?;
    let cache = auth.session_info().cache();

    if let Some((session, user)) = cache.get(&token) {
        if session.expires < now {
            cache.invalidate(&token);

            return Err(LookupError::SessionExpired(session));
        }

        if !session.authenticated {
            return Err(LookupError::SessionUnauthenticated(session));
        }

        if !session.verified {
            return Err(LookupError::SessionUnverified(session));
        }

        Ok(Initiator {
            user,
            mechanism: Mechanism::Session(session)
        })
    } else if let Some(session) = session::Session::retrieve_token(conn, &token).await? {
        if session.dropped || session.expires < now {
            return Err(LookupError::SessionExpired(session));
        }

        if !session.authenticated {
            return Err(LookupError::SessionUnauthenticated(session));
        }

        if !session.verified {
            return Err(LookupError::SessionUnverified(session));
        }

        if let Some(user) = user::User::query_with_id(conn, &session.user_id).await? {
            cache.insert(token, (session.clone(), user.clone()));

            Ok(Initiator {
                user,
                mechanism: Mechanism::Session(session),
            })
        } else {
            Err(LookupError::UserNotFound(Mechanism::Session(session)))
        }
    } else {
        Err(LookupError::SessionNotFound)
    }
}

fn find_session_id<'a>(cookies: GetAll<'a, HeaderValue>) -> Result<Option<&'a str>, LookupError> {
    for value in cookies {
        let value_str = value.to_str()?;

        if let Some((name, value)) = value_str.split_once('=') {
            if name == "session_id" {
                return Ok(Some(value));
            }
        }
    }

    Ok(None)
}

pub async fn lookup_header_map(
    auth: &state::Sec,
    conn: &impl GenericClient,
    headers: &HeaderMap
) -> Result<Initiator, LookupError> {
    let cookies = headers.get_all("cookie");

    if let Some(found) = find_session_id(cookies)? {
        return lookup_session_id(auth, conn, found.as_bytes()).await;
    }

    Err(LookupError::MechanismNotFound)
}

impl FromRequestParts<ArcShared> for Initiator {
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 ArcShared,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait
    {
        let sec = state.sec();
        let fut = state.pool().get();

        Box::pin(async move {
            let conn = match fut.await {
                Ok(obj) => obj,
                Err(err) => return Err(ApiError::from(err)
                    .context("failed to retrieve database connection"))
            };

            Ok(lookup_header_map(sec, &conn, &parts.headers).await?)
        })
    }
}
