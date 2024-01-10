use http::StatusCode;
use axum_core::response::{Response, IntoResponse};
use serde::{Serialize, Deserialize};

use crate::response::{serialize_json, error_json};

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    payload: T
}

impl<T> Payload<T> {
    pub fn new(payload: T) -> Self {
        Self {
            message: None,
            payload
        }
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().map(|v| v.as_str())
    }

    pub fn set_message<M>(&mut self, msg: M)
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
        self
    }

    pub fn set_payload(&mut self, payload: T) {
        self.payload = payload;
    }

    pub fn with_payload(mut self, payload: T) -> Self {
        self.payload = payload;
        self
    }

    pub fn swap_payload<P>(self, payload: P) -> Payload<P> {
        Payload {
            message: self.message,
            payload
        }
    }

    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T> std::fmt::Display for Payload<T>
where
    T: std::fmt::Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.message {
            Some(message) => {
                if f.alternate() {
                    write!(f, "{} -> {:#}", message, self.payload)
                } else {
                    write!(f, "{} -> {}", message, self.payload)
                }
            },
            None => {
                if f.alternate() {
                    write!(f, "{:#}", self.payload)
                } else {
                    write!(f, "{}", self.payload)
                }
            }
        }
    }
}

impl<T> IntoResponse for Payload<T>
where
    T: Serialize
{
    fn into_response(self) -> Response {
        match serialize_json(StatusCode::OK, &self) {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("Payload<T> serialization error {:?}", err);
                error_json()
            }
        }
    }
}
