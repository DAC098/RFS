use std::fmt::Debug;

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

pub struct Json<T> {
    builder: Builder,
    root: T
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

impl<T> Json<rfs_lib::json::Wrapper<T>> {
    pub fn with_kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.root = self.root.with_kind(kind);
        self
    }

    pub fn with_message<M>(mut self, message: M) -> Self
    where
        M: Into<String>
    {
        self.root = self.root.with_message(message);
        self
    }

    pub fn with_payload<P>(self, payload: P) -> Json<rfs_lib::json::Wrapper<P>> {
        Json {
            builder: self.builder,
            root: self.root.with_payload(payload)
        }
    }
}

impl Json<rfs_lib::json::Wrapper<()>> {
    pub fn empty() -> Self {
        Self {
            builder: Builder::new(),
            root: rfs_lib::json::Wrapper::new(()),
        }
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

