use std::marker::PhantomData;

use axum::http::StatusCode;
use axum::response::{Response, IntoResponse};
use axum::body::BoxBody;
use tracing::Level;

type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
struct Inner {
    status: StatusCode,
    kind: String,
    msg: Option<String>,
    src: Option<BoxDynError>,
}

pub trait ErrorResponse {
    fn into_response(inner: Inner) -> Response;
}

#[derive(Debug)]
pub struct Text {}

impl ErrorResponse for Text {
    fn into_response(inner: Inner) -> Response {
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
}

#[derive(Debug)]
pub struct Json {}

impl ErrorResponse for Json {
    fn into_response(inner: Inner) -> Response {
        let mut body = String::new();
        body.push_str("{\"error\":\"");
        body.push_str(&inner.kind);
        body.push('"');

        if let Some(msg) = inner.msg {
            body.push_str(",\"message\":\"");
            body.push_str(&msg);
            body.push('"');
        }

        body.push('}');

        Response::builder()
            .status(inner.status)
            .header("content-type", "application/json")
            .header("content-length", body.len())
            .body(body)
            .unwrap()
            .into_response()
    }
}

#[derive(Debug)]
pub struct Error<R = Text> {
    inner: Inner,
    phantom: PhantomData<R>
}

pub type Result<T, R = Text> = std::result::Result<T, Error<R>>;

impl<R> Error<R> {
    pub fn new() -> Self {
        Error {
            inner: Inner {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                kind: String::from("InternalServerError"),
                msg: None,
                src: None,
            },
            phantom: PhantomData
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

impl<R> std::fmt::Display for Error<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.kind)?;

        if let Some(msg) = self.inner.msg.as_ref() {
            write!(f, ": {}", msg)?;
        }

        Ok(())
    }
}

impl<R> std::error::Error for Error<R>
where
    R: std::fmt::Debug
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.src.as_ref().map(|v| & **v as _)
    }
}

impl<R> axum::response::IntoResponse for Error<R>
where
    R: ErrorResponse
{
    fn into_response(self) -> axum::response::Response {
        if let Some(err) = self.inner.src.as_ref() {
            tracing::event!(
                Level::ERROR,
                "error during request {:#?}",
                err
            );
        }

        R::into_response(self.inner).into_response()
    }
}

macro_rules! simple_from {
    ($e:path) => {
        impl<R> From<$e> for Error<R> {
            fn from(err: $e) -> Self {
                Error::new()
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr) => {
        impl<R> From<$e> for Error<R> {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr, $m:expr) => {
        impl<R> From<$e> for Error<R> {
            fn from(err: $e) -> Self {
                Error::new()
                    .kind($k)
                    .message($m)
                    .source(err)
            }
        }
    };
    ($e:path, $k:expr, $m:expr, $s:expr) => {
        impl<R> From<$e> for Error<R> {
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

simple_from!(axum::http::Error);
simple_from!(axum::http::header::ToStrError);

simple_from!(handlebars::RenderError);
