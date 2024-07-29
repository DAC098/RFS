use std::default::Default;
use std::fmt::{Write, Formatter, Display, Debug, Result as FmtResult};

mod base;

use base::{Er, BoxDynError};

pub mod api;

pub use api::{Error as ApiError, Result as ApiResult};

pub struct StrError(pub String);

impl Default for StrError {
    fn default() -> Self {
        StrError(String::from("Error"))
    }
}

impl Display for StrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Debug for StrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.0, f)
    }
}

pub type Error = Er<StrError>;
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    pub fn kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.inner = StrError(kind.into());
        self
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::default()
            .context(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::default()
            .context(msg)
    }
}

impl From<deadpool_postgres::BuildError> for Error {
    fn from(err: deadpool_postgres::BuildError) -> Self {
        use deadpool_postgres::BuildError;

        match err {
            BuildError::Backend(e) => Error::default()
                .source(e),
            BuildError::NoRuntimeSpecified(string) => Error::default()
                .source(string)
        }
    }
}

impl From<deadpool_postgres::HookErrorCause> for Error {
    fn from(err: deadpool_postgres::HookErrorCause) -> Self {
        use deadpool_postgres::HookErrorCause;

        match err {
            HookErrorCause::Backend(e) => Self::from(e),
            HookErrorCause::Message(msg) => Error::default()
                .source(msg),
            HookErrorCause::StaticMessage(msg) => Error::default()
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
                    Error::default()
                        .context("no error")
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
            _ => Error::default().source(err)
        }
    }
}

impl From<hkdf::InvalidLength> for Error {
    fn from(_err: hkdf::InvalidLength) -> Self {
        Error::default()
            .source("invalid output length when deriving key")
    }
}

macro_rules! generic_catch {
    ($k:expr, $e:path) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::default()
                    .kind($k)
                    .source(err)
            }
        }
    };
    ($k:expr, $e:path, $m:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::default()
                    .kind($k)
                    .context($m)
                    .source(err)
            }
        }
    }
}

generic_catch!("std::io::Error", std::io::Error);
generic_catch!("std::net::AddrParseError", std::net::AddrParseError);
generic_catch!("handlebars::TemplateError", handlebars::TemplateError);
generic_catch!("tokio_postgres::Error", tokio_postgres::Error);
generic_catch!("snowcloud_cloud::error::Error", snowcloud_cloud::error::Error);
generic_catch!("serde_json::Error", serde_json::Error);
generic_catch!("serde_yaml::Error", serde_yaml::Error);
generic_catch!("rand::Error", rand::Error);

pub trait Context<T, E> {
    fn context<C>(self, cxt: C) -> std::result::Result<T, Error>
    where
        C: Into<String>;

    fn kind<K>(self, kind: K) -> std::result::Result<T, Error>
    where
        K: Into<String>;
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
            Err(err) => Err(Error::default()
                .context(cxt)
                .source(err))
        }
    }

    fn kind<K>(self, kind: K) -> std::result::Result<T, Error>
    where
        K: Into<String>
    {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(Error {
                inner: StrError(kind.into()),
                cxt: None,
                src: Some(err.into())
            })
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
            None => Err(Error::default()
                .context(cxt))
        }
    }

    fn kind<K>(self, kind: K) -> std::result::Result<T, Error>
    where
        K: Into<String>
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Error {
                inner: StrError(kind.into()),
                cxt: None,
                src: None
            })
        }
    }
}

pub fn trace_error<D, E>(prefix: &D, error: &E)
where
    D: Display + ?Sized,
    E: std::error::Error
{
    let mut msg_failed = false;
    let mut msg = format!("0) {error}");
    let mut count = 1;
    let mut curr = std::error::Error::source(error);

    while let Some(next) = curr {
        if let Err(err) = write!(&mut msg, "\n{count}) {next}") {
            tracing::error!("error when attempting to create error trace\n{err}");

            msg_failed = true;

            break;
        }

        count += 1;
        curr = std::error::Error::source(next);
    }

    if !msg_failed {
        tracing::error!("{prefix}\n{msg}");
    }
}
