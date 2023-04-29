use bytes::{BufMut, BytesMut};
use axum::body::{Bytes};
use axum::http::StatusCode;
use axum::response::{Response, IntoResponse};
use serde::Serialize;

pub mod error;
pub mod mime;

pub mod fs;
pub mod html;

#[derive(Serialize)]
struct JsonBody<T> {
    message: Option<String>,
    payload: T
}

pub struct Json<T> {
    status: StatusCode,
    msg: Option<String>,
    pld: T,
}

impl Json<()> {
    pub fn empty() -> Self {
        Json {
            status: StatusCode::OK,
            msg: None,
            pld: ()
        }
    }
}

impl<T> Json<T> {
    pub fn new(payload: T) -> Self {
        Json {
            status: StatusCode::OK,
            msg: None,
            pld: payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.pld
    }

    pub fn message(&self) -> &Option<String> {
        &self.msg
    }

    pub fn set_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn set_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.msg = Some(msg.into());
        self
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize
{
    fn into_response(self) -> Response {
        let json = JsonBody {
            message: self.msg,
            payload: self.pld
        };

        let mut buf = BytesMut::with_capacity(128).writer();

        match serde_json::to_writer(&mut buf, &json) {
            Ok(()) => {
                (
                    self.status,
                    [("content-type", "application/json")],
                    buf.into_inner().freeze()
                ).into_response()
            },
            Err(err) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "error when serializing json {:#?}",
                    err
                );

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [("content-type", "text/plain")],
                    String::from("failed to serialize data to json")
                ).into_response()
            }
        }
    }
}

