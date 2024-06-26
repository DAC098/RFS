type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    kind: String,
    msg: Option<String>,
    src: Option<BoxDynError>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new() -> Error {
        Error {
            kind: String::from("Error"),
            msg: None,
            src: None,
        }
    }

    pub fn kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.kind = kind.into();
        self
    }

    pub fn message<M>(mut self, msg: M) -> Error
    where
        M: Into<String>
    {
        self.msg = Some(msg.into());
        self
    }

    pub fn source<S>(mut self, src: S) -> Error
    where
        S: Into<BoxDynError>
    {
        self.src = Some(src.into());
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.kind, self.msg.as_ref(), self.src.as_ref()) {
            (kind, Some(msg), Some(err)) => {
                write!(f, "{kind}: {msg}\n{err}")
            },
            (kind, Some(msg), None) => {
                write!(f, "{kind}: {msg}")
            },
            (kind, None, Some(err)) => {
                write!(f, "{kind}: {err}")
            },
            (kind, None, None) => {
                write!(f, "{kind}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_ref().map(|v| & **v as _)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::new()
            .message(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::new()
            .message(msg)
    }
}

impl From<deadpool_postgres::BuildError> for Error {
    fn from(err: deadpool_postgres::BuildError) -> Self {
        use deadpool_postgres::BuildError;

        match err {
            BuildError::Backend(e) => Error::new()
                .kind("tokio_postgres::Error")
                .source(e),
            BuildError::NoRuntimeSpecified(string) => Error::new()
                .kind("deadpool::managed::BuildError")
                .source(string)
        }
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

impl From<hkdf::InvalidLength> for Error {
    fn from(_err: hkdf::InvalidLength) -> Self {
        Error::new()
            .kind("hkdf::InvalidLength")
            .source("invalid output length when deriving key")
    }
}

macro_rules! generic_catch {
    ($k:expr, $e:path) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .source(err)
            }
        }
    };
    ($k:expr, $e:path, $m:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .message($m)
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

use rfs_lib::context_trait;

context_trait!(Error);

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
                .message(cxt)
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
                .message(cxt))
        }
    }
}
