use axum::response::{Response, IntoResponse};
use tracing::Level;

use super::base::{Er, BoxDynError};

pub use rfs_api::error::{
    Detail,
    ApiErrorKind,
    ApiError
};

pub async fn handle_error<E>(error: E) -> Response
where
    E: Into<Error>
{
    let error = error.into();

    if let Some(err) = error.src.as_ref() {
        tracing::event!(
            Level::ERROR,
            "unhandled error when prcessing request: {:#?}",
            err
        );
    }

    error.inner.into_response()
}

pub type Error = Er<ApiError>;
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new() -> Self {
        Error {
            inner: Default::default(),
            cxt: None,
            src: None,
        }
    }

    pub fn api<T>(value: T) -> Self
    where
        T: Into<ApiError>
    {
        Error {
            inner: value.into(),
            cxt: None,
            src: None
        }
    }

    pub fn kind(mut self, kind: ApiErrorKind) -> Self {
        self.inner = self.inner.with_kind(kind);
        self
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        if let Some(err) = self.src.as_ref() {
            tracing::event!(
                Level::ERROR,
                "unhandled error when prcessing request: {:#?}",
                err
            );
        }

        self.inner.into_response()
    }
}

impl From<ApiError> for Error {
    fn from(api_err: ApiError) -> Self {
        Error {
            inner: api_err,
            cxt: None,
            src: None,
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_infallible: std::convert::Infallible) -> Self {
        // this should not happen
        Error::new()
            .source("Infallible. how did this happen")
    }
}

impl From<deadpool_postgres::HookErrorCause> for Error {
    fn from(err: deadpool_postgres::HookErrorCause) -> Self {
        use deadpool_postgres::HookErrorCause;

        match err {
            HookErrorCause::Backend(e) => Self::from(e),
            HookErrorCause::Message(msg) => Error::new()
                .source(msg),
            HookErrorCause::StaticMessage(msg) => Error::new()
                .source(msg.to_owned()),
        }
    }
}

impl From<deadpool_postgres::HookError> for Error {
    fn from(err: deadpool_postgres::HookError) -> Self {
        use deadpool_postgres::HookError;

        match err {
            HookError::Continue(opt) => {
                if let Some(cause) = opt {
                    Self::from(cause)
                } else {
                    Error::new()
                        .source("deadpool::managed::HookError::Continue with no cause")
                }
            },
            HookError::Abort(cause) => {
                Self::from(cause)
            }
        }
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        use deadpool_postgres::PoolError;

        match err {
            PoolError::Backend(e) => Self::from(e),
            PoolError::PostCreateHook(e) |
            PoolError::PreRecycleHook(e) |
            PoolError::PostRecycleHook(e) => Self::from(e),
            _ => Error::new().source(err)
        }
    }
}

macro_rules! simple_from {
    ($e:path) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr, $m:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .context($m)
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr, $m:expr, $s:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .status($s)
                    .kind($k)
                    .context($m)
                    .source(err)
            }
        }
    }
}

simple_from!(rfs_lib::sec::chacha::CryptoError);

simple_from!(std::io::Error);
simple_from!(std::fmt::Error);

simple_from!(axum::Error);
simple_from!(axum::http::Error);
simple_from!(
    axum::http::header::ToStrError,
    ApiErrorKind::InvalidHeaderValue
);
simple_from!(
    axum::http::header::InvalidHeaderValue,
    ApiErrorKind::InvalidHeaderValue
);

simple_from!(
    mime::FromStrError,
    ApiErrorKind::InvalidMimeType
);

simple_from!(handlebars::RenderError);

simple_from!(tokio_postgres::Error);

simple_from!(serde_json::Error);

simple_from!(rand::Error);

simple_from!(argon2::Error);

simple_from!(rust_otp::error::Error);

simple_from!(snowcloud_cloud::error::Error);

simple_from!(rust_kms_local::local::Error);

// ----------------------------------------------------------------------------

pub trait Context<T, E> {
    fn context<C>(self, cxt: C) -> std::result::Result<T, Error>
    where
        C: Into<String>;

    fn kind(self, kind: ApiErrorKind) -> std::result::Result<T, Error>;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: Into<BoxDynError>
{
    fn context<C>(self, cxt: C) -> std::result::Result<T, Error>
    where
        C: Into<String>
    {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::new()
                .context(cxt)
                .source(err))
        }
    }

    fn kind(self, kind: ApiErrorKind) -> std::result::Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::new()
                .kind(kind)
                .source(err))
        }
    }
}

impl<T> Context<T, ()> for std::option::Option<T> {
    fn context<C>(self, cxt: C) -> std::result::Result<T, Error>
    where
        C: Into<String>
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::new()
                .context(cxt))
        }
    }

    fn kind(self, kind: ApiErrorKind) -> std::result::Result<T, Error> {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::new()
                .kind(kind))
        }
    }
}
