use rfs_api::error::ApiErrorKind;
use bytes::{BufMut, BytesMut};
use axum::body::Full;
use axum::http::StatusCode;
use axum::http::header::{HeaderMap};
use axum::response::{Response, IntoResponse};
use tracing::Level;

pub use rfs_api::error::{
    ApiError,
    Detail,
    GeneralKind,
    StorageKind,
    FsKind,
    TagKind,
    UserKind,
    AuthKind,
    SecKind
};

type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

pub fn error_json_response(status: StatusCode, error: ApiError) -> Response {
    let buf = {
        let mut buf = BytesMut::with_capacity(128).writer();
        serde_json::to_writer(&mut buf, &error).unwrap();

        buf.into_inner().freeze()
    };

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("content-length", buf.len())
        .body(Full::new(buf))
        .unwrap()
        .into_response()
}

pub async fn handle_error<E>(
    _headers: HeaderMap,
    error: E
) -> Response
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

    error_json_response(error.status, error.inner)
}

#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    inner: ApiError,
    src: Option<BoxDynError>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new() -> Self {
        let inner = ApiError::from(GeneralKind::InternalFailure);
        let status = inner.kind().into();

        Error {
            status,
            inner,
            src: None,
        }
    }

    pub fn api<T>(value: T) -> Self
    where
        T: Into<ApiError>
    {
        let err = value.into();
        let status = err.kind().into();

        Error {
            status,
            inner: err,
            src: None
        }
    }

    #[allow(dead_code)]
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    #[allow(dead_code)]
    pub fn kind<K>(mut self, kind: K) -> Self
    where
        K: Into<ApiErrorKind>
    {
        self.inner = self.inner.with_kind(kind);
        self
    }

    #[allow(dead_code)]
    pub fn detail(mut self, detail: Detail) -> Self {
        self.inner = self.inner.with_detail(detail);
        self
    }

    #[allow(dead_code)]
    pub fn message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.inner = self.inner.with_message(msg.into());
        self
    }

    pub fn source<S>(mut self, src: S) -> Self
    where
        S: Into<BoxDynError>
    {
        self.src = Some(src.into());
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.kind())?;

        if let Some(msg) = self.inner.message() {
            write!(f, ": {}", msg)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_ref().map(|v| & **v as _)
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

        error_json_response(self.status, self.inner)
    }
}

impl From<ApiError> for Error {
    fn from(api_err: ApiError) -> Self {
        let status = api_err.kind().into();

        Error {
            status,
            inner: api_err,
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
                    //.kind($k)
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr, $m:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    //.kind($k)
                    .message($m)
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr, $m:expr, $s:expr) => {
        impl From<$e> for Error {
            fn from(err: $e) -> Self {
                Error::new()
                    .status($s)
                    //.kind($k)
                    .message($m)
                    .source(err)
            }
        }
    }
}

simple_from!(std::io::Error);
simple_from!(std::fmt::Error);

simple_from!(axum::Error);
simple_from!(axum::http::Error);
simple_from!(
    axum::http::header::ToStrError,
    GeneralKind::InvalidHeaderValue
);

simple_from!(
    mime::FromStrError,
    GeneralKind::InvalidMimeType
);

simple_from!(handlebars::RenderError);

simple_from!(tokio_postgres::Error);

simple_from!(serde_json::Error);

simple_from!(rand::Error);

simple_from!(argon2::Error);

simple_from!(rust_otp::error::Error);

simple_from!(snowcloud_cloud::error::Error);

simple_from!(rust_kms_local::local::Error);
