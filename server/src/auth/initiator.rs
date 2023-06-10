use std::ops::Deref;
use std::pin::Pin;
use std::future::Future;

use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderValue, GetAll};
use axum::http::request::Parts;
use axum::extract::FromRequestParts;
use deadpool_postgres::{Pool, GenericClient};

use crate::net::error;
use super::state;
use super::session;
use crate::user;

pub enum Authorization {
    Session(session::Session),
    Jwt(()),
}

pub struct Initiator {
    user: user::User,
    bot: Option<()>,
    access: Authorization
}

impl Initiator {
    pub fn user(&self) -> &user::User {
        &self.user
    }

    pub fn bot(&self) -> Option<&()> {
        self.bot.as_ref()
    }

    pub fn access(&self) -> &Authorization {
        &self.access
    }
}

pub enum LookupError {
    InvalidString,
    InvalidLength,
    InvalidHash,
    SessionNotFound,
    SessionExpired(session::Session),
    SessionUnauthenticated(session::Session),
    SessionUnverified(session::Session),

    UserNotFound(Authorization),

    AuthorizationNotFound,

    Database(tokio_postgres::Error),
    HeaderToStr(axum::http::header::ToStrError),
}

impl From<tokio_postgres::Error> for LookupError {
    fn from(e: tokio_postgres::Error) -> Self {
        LookupError::Database(e)
    }
}

impl From<axum::http::header::ToStrError> for LookupError {
    fn from(e: axum::http::header::ToStrError) -> Self {
        LookupError::HeaderToStr(e)
    }
}

impl From<LookupError> for error::Error {
    fn from(e: LookupError) -> Self {
        match e {
            LookupError::InvalidString |
            LookupError::InvalidLength |
            LookupError::InvalidHash => error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("InvalidSession")
                .message("session id is invalid"),
            LookupError::SessionNotFound => error::Error::new()
                .status(StatusCode::NOT_FOUND)
                .kind("SessionNotFound")
                .message("session was not found"),
            LookupError::SessionExpired(session) => error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("SessionExpired")
                .message("session has expired"),
            LookupError::SessionUnauthenticated(session) => error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("SessionUnauthenticated")
                .message("session has not been authenticated"),
            LookupError::SessionUnverified(session) => error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("SessionUnverified")
                .message("session has not been verified"),

            LookupError::UserNotFound(authorization) => error::Error::new()
                .status(StatusCode::NOT_FOUND)
                .kind("UserNotFound")
                .message("authorization user was not found"),

            LookupError::AuthorizationNotFound => error::Error::new()
                .status(StatusCode::UNAUTHORIZED)
                .kind("AuthorizationNotFound")
                .message("authorization not provided"),

            LookupError::Database(e) => e.into(),
            LookupError::HeaderToStr(e) => e.into(),
        }
    }
}

pub async fn lookup_session_id<S>(
    auth: &state::Auth,
    conn: &impl GenericClient,
    session_id: S
) -> Result<Initiator, LookupError>
where
    S: AsRef<[u8]>
{
    let (token, hash) = match session::decode_base64(auth, session_id) {
        Ok(p) => p,
        Err(err) => {
            return Err(match err {
                session::DecodeError::InvalidString => LookupError::InvalidString,
                session::DecodeError::InvalidLength => LookupError::InvalidLength,
                session::DecodeError::InvalidHash => LookupError::InvalidHash
            })
        }
    };

    if let Some(session) = session::Session::query_with_token(conn, &token).await? {
        let now = chrono::Utc::now();

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
            Ok(Initiator {
                user,
                bot: None,
                access: Authorization::Session(session),
            })
        } else {
            Err(LookupError::UserNotFound(Authorization::Session(session)))
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
    auth: &state::Auth,
    conn: &impl GenericClient,
    headers: &HeaderMap
) -> Result<Initiator, LookupError> {
    tracing::event!(
        tracing::Level::DEBUG,
        "headers: {:?}",
        headers
    );

    let cookies = headers.get_all("cookie");

    if let Some(found) = find_session_id(cookies)? {
        return lookup_session_id(auth, conn, found.as_bytes()).await;
    }

    Err(LookupError::AuthorizationNotFound)
}

impl<A, S> FromRequestParts<A> for Initiator
where
    A: Deref<Target = S> + Sync,
    S: AsRef<state::Auth> + AsRef<Pool> + Sync,
{
    type Rejection = error::Error;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 A,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait
    {
        Box::pin(async move {
            // since we are not explicitly requireing crate::state::Shared
            // we can still achieve what we want by using Deref and then
            // get the auth and pool structs
            // this is wierd but it works so...
            let state_deref = state.deref();

            let auth: &state::Auth = state_deref.as_ref();
            let pool: &Pool = state_deref.as_ref();
            let conn = pool.get().await?;

            Ok(lookup_header_map(auth, &conn, &parts.headers).await?)
        })
    }
}

