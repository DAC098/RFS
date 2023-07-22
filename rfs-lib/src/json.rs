use std::fmt::Debug;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Wrapper<T> {
    kind: Option<String>,
    message: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    payload: T
}

impl<T> Wrapper<T> {
    pub fn new(payload: T) -> Self {
        Self {
            kind: None,
            message: None,
            timestamp: None,
            payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn kind(&self) -> Option<&String> {
        self.kind.as_ref()
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn timestamp(&self) -> Option<&DateTime<Utc>> {
        self.timestamp.as_ref()
    }

    pub fn with_timestamp_now(mut self) -> Self {
        self.timestamp = Some(Utc::now());
        self
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
        self
    }

    pub fn with_kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_payload<P>(self, payload: P) -> Wrapper<P> {
        Wrapper {
            kind: self.kind,
            message: self.message,
            timestamp: self.timestamp,
            payload
        }
    }

    pub fn into_payload(self) -> T {
        self.payload
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListWrapper<T> {
    kind: Option<String>,
    message: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    total: usize,
    payload: T,
}

impl<T> ListWrapper<T> {
    pub fn new(payload: T) -> Self {
        Self {
            kind: None,
            message: None,
            timestamp: None,
            total: 0,
            payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn kind(&self) -> Option<&String> {
        self.kind.as_ref()
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn timestamp(&self) -> Option<&DateTime<Utc>> {
        self.timestamp.as_ref()
    }

    pub fn with_timestamp_now(mut self) -> Self {
        self.timestamp = Some(Utc::now());
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

    pub fn with_kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
        self
    }

    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T> ListWrapper<Vec<T>> {
    pub fn with_vec(vec: Vec<T>) -> Self {
        Self {
            kind: None,
            message: None,
            timestamp: None,
            total: vec.len(),
            payload: vec
        }
    }

    pub fn with_slice(slice: &[T]) -> Self
    where
        T: Clone
    {
        Self {
            kind: None,
            message: None,
            timestamp: None,
            total: slice.len(),
            payload: slice.to_vec(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    kind: String,
    message: Option<String>
}

impl Error {
    pub fn new<K>(kind: K) -> Self
    where
        K: Into<String>
    {
        Self {
            kind: kind.into(),
            message: None
        }
    }

    pub fn kind(&self) -> &String {
        &self.kind
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn set_message<M>(&mut self, message: M)
    where
        M: Into<String>
    {
        self.message = Some(message.into());
    }

    pub fn with_message<M>(mut self, message: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(message.into());
        self
    }
}
