use bytes::{BufMut, BytesMut};
use axum::body::Full;
use axum::http::{
    StatusCode,
    HeaderName,
    HeaderValue,
    Error as HttpError,
    response::Builder,
};
use axum::response::{Response, IntoResponse};
use serde::Serialize;

pub mod error;
pub mod mime;
pub mod cookie;
pub mod layer;

pub mod fs;
pub mod html;

#[derive(Serialize)]
pub struct JsonWrapper<T> {
    message: Option<String>,
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
    payload: T
}

impl<T> JsonWrapper<T> {
    pub fn new(payload: T) -> Self {
        Self {
            message: None,
            timestamp: None,
            payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn message(&self) -> &Option<String> {
        &self.message
    }

    pub fn timestamp(&self) -> &Option<chrono::DateTime<chrono::Utc>> {
        &self.timestamp
    }

    pub fn with_timestamp_now(mut self) -> Self {
        self.timestamp = Some(chrono::Utc::now());
        self
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
        self
    }
}

#[derive(Serialize)]
pub struct JsonListWrapper<T> {
    message: Option<String>,
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
    total: usize,
    payload: T,
}

impl<T> JsonListWrapper<T> {
    pub fn new(payload: T) -> Self {
        Self {
            message: None,
            timestamp: None,
            total: 0,
            payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn with_timestamp_now(mut self) -> Self {
        self.timestamp = Some(chrono::Utc::now());
        self
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>,
    {
        self.message = Some(msg.into());
        self
    }

    pub fn with_total(mut self, total: usize) -> Self {
        self.total = total;
        self
    }
}

impl<T> JsonListWrapper<Vec<T>> {
    pub fn with_vec(payload: Vec<T>) -> Self {
        Self {
            message: None,
            timestamp: None,
            total: payload.len(),
            payload
        }
    }
}

pub struct Json<T> {
    builder: Builder,
    root: T,
}

impl<T> Json<JsonWrapper<T>> {
    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.root.message = Some(msg.into());
        self
    }

    pub fn with_timestamp_now(mut self) -> Self {
        self.root.timestamp = Some(chrono::Utc::now());
        self
    }
}

impl Json<JsonWrapper<()>> {
    pub fn empty() -> Self {
        Json::new(JsonWrapper::new(()))
    }
}

impl<T> Json<T> {
    pub fn new(root: T) -> Self {
        Self {
            builder: Builder::new(),
            root
        }
    }

    pub fn root(&self) -> &T {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut T {
        &mut self.root
    }

    pub fn with_status<S>(mut self, status: S) -> Self
    where
        StatusCode: TryFrom<S>,
        <StatusCode as TryFrom<S>>::Error: Into<HttpError>
    {
        self.builder = self.builder.status(status);
        self
    }

    pub fn with_header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<HttpError>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<HttpError>,
    {
        self.builder = self.builder.header(key, value);
        self
    }
}

impl<T> Json<T>
where
    T: Serialize
{
    fn create_response(self) -> error::Result<Response> {
        let buf_froze = {
            let mut buf = BytesMut::with_capacity(128).writer();
            serde_json::to_writer(&mut buf, &self.root)?;

            buf.into_inner().freeze()
        };

        Ok(self.builder.header("content-type", "application/json")
            .body(Full::new(buf_froze))?
            .into_response())
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize
{
    fn into_response(self) -> Response {
        match self.create_response() {
            Ok(res) => res,
            Err(err) => err.into_response(),
        }
    }
}

