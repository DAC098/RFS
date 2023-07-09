use bytes::{BufMut, BytesMut};
use axum::body::Full;
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, ACCEPT};
use axum::response::{Response, IntoResponse};
use tracing::Level;

const DEFAULT_ACCEPT_VALUE: &str = "application/json";

type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

fn box_into_inner<T>(b: Box<T>) -> T {
    *b
}

fn handle_error_json(inner: Inner) -> Response {
    let mut json = lib::json::Error::new(inner.kind);

    if let Some(msg) = inner.msg {
        json.set_message(msg);
    }

    let buf = {
        let mut buf = BytesMut::with_capacity(128).writer();
        serde_json::to_writer(&mut buf, &json).unwrap();

        buf.into_inner().freeze()
    };

    Response::builder()
        .status(inner.status)
        .header("content-type", "application/json")
        .header("content-length", buf.len())
        .body(Full::new(buf))
        .unwrap()
        .into_response()
}

fn handle_error_text(inner: Inner) -> Response {
    let mut body = inner.kind;

    if let Some(msg) = inner.msg {
        body.push_str(": ");
        body.push_str(&msg);
    }

    Response::builder()
        .status(inner.status)
        .header("content-type", "text/plain")
        .header("content-length", body.len())
        .body(body)
        .unwrap()
        .into_response()
}

pub async fn handle_error<E>(
    headers: HeaderMap,
    error: E
) -> Response
where
    E: Into<Error>
{
    let error = error.into();

    if let Some(err) = error.inner.src.as_ref() {
        tracing::event!(
            Level::ERROR,
            "unhandled error when prcessing request: {:#?}",
            err
        );
    }

    let accept_value = if let Some(found) = headers.get(ACCEPT) {
        let Ok(p) = found.to_str() else {
            return handle_error_json(error.inner);
        };

        p
    } else {
        DEFAULT_ACCEPT_VALUE
    };

    let mut iter = mime::MimeIter::new(accept_value);

    while let Some(check) = iter.next() {
        if let Ok(part) = check {
            if part.type_() == "text" {
                if part.subtype() == "plain" {
                    return handle_error_text(error.inner);
                }
            } else if part.type_() == "application" {
                if part.subtype() == "json" {
                    return handle_error_json(error.inner);
                }
            }
        }
    }

    handle_error_json(error.inner)
}

#[derive(Debug)]
pub struct Inner {
    status: StatusCode,
    kind: String,
    msg: Option<String>,
    src: Option<BoxDynError>,
}

#[derive(Debug)]
pub struct Error {
    inner: Inner,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new() -> Self {
        Error {
            inner: Inner {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                kind: String::from("InternalServerError"),
                msg: None,
                src: None,
            },
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.inner.status = status;
        self
    }

    pub fn kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.inner.kind = kind.into();
        self
    }

    pub fn message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.inner.msg = Some(msg.into());
        self
    }

    pub fn source<S>(mut self, src: S) -> Self
    where
        S: Into<BoxDynError>
    {
        self.inner.src = Some(src.into());
        self
    }

}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.kind)?;

        if let Some(msg) = self.inner.msg.as_ref() {
            write!(f, ": {}", msg)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.src.as_ref().map(|v| & **v as _)
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        if let Some(err) = self.inner.src.as_ref() {
            tracing::event!(
                Level::ERROR,
                "unhandled error when prcessing request: {:#?}",
                err
            );
        }

        handle_error_json(self.inner)
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(infallible: std::convert::Infallible) -> Self {
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
                    .kind($k)
                    .message($m)
                    .source(err)
            }
        }
    }
}

simple_from!(std::io::Error);

simple_from!(axum::Error);
simple_from!(axum::http::Error);
simple_from!(
    axum::http::header::ToStrError,
    "InvalidHeaderValue",
    "header value contained non utf-8 characters",
    StatusCode::BAD_REQUEST
);

simple_from!(
    mime::FromStrError,
    "InvalidMimeType",
    "given invalid mime type value",
    StatusCode::BAD_REQUEST
);

simple_from!(handlebars::RenderError);

simple_from!(tokio_postgres::Error);

simple_from!(serde_json::Error);

simple_from!(rand::Error);

simple_from!(argon2::Error);

simple_from!(rust_otp::error::Error);

simple_from!(snowcloud_cloud::error::Error);
