use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Wrapper<T> {
    message: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    payload: T
}

impl<T> Wrapper<T> {
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

    pub fn into_payload(self) -> T {
        self.payload
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListWrapper<T> {
    message: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    total: usize,
    payload: T,
}

impl<T> ListWrapper<T> {
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

    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T> ListWrapper<Vec<T>> {
    pub fn with_vec(vec: Vec<T>) -> Self {
        Self {
            message: None,
            timestamp: None,
            total: vec.len(),
            payload: vec
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    error: String,
    message: Option<String>
}

impl Error {
    pub fn new<K>(kind: K) -> Self
    where
        K: Into<String>
    {
        Self {
            error: kind.into(),
            message: None
        }
    }

    pub fn error(&self) -> &String {
        &self.error
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn with_message<M>(mut self, message: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(message.into());
        self
    }
}
