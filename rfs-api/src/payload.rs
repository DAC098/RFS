use http::StatusCode;
use axum_core::body::Body;
use axum_core::response::{Response, IntoResponse};
use serde::{Serialize, Deserialize};
use bytes::{BytesMut, BufMut};

fn serialize_to_json(
    data: &impl Serialize
) -> Result<Response, serde_json::Error> {
    let froze = {

        let mut buf = BytesMut::with_capacity(128).writer();
        serde_json::to_writer(&mut buf, data)?;

        buf.into_inner().freeze()
    };

    Ok(Response::builder()
       .status(StatusCode::OK)
       .header("content-type", "applicatin/json")
       .header("content-length", froze.len())
       .body(Body::from(froze))
       .unwrap())
}

fn error_json() -> Response {
    let body = r#"{"kind":"InternalFailure"}"#;

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "applicaiton/json")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    payload: T
}

impl<T> Payload<T> {
    pub fn new(payload: T) -> Self {
        Self {
            kind: None,
            message: None,
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

    pub fn set_message<M>(&mut self, msg: M) -> ()
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

    pub fn set_kind<K>(&mut self, kind: K) -> ()
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
    }

    pub fn with_kind<K>(mut self, kind: K) -> Self
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_payload<P>(self, payload: P) -> Payload<P> {
        Payload {
            kind: self.kind,
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
        match (&self.kind, &self.message) {
            (Some(kind), Some(message)) => {
                if f.alternate() {
                    write!(f, "{}: {} -> {:#}", kind, message, self.payload)
                } else {
                    write!(f, "{}: {} -> {}", kind, message, self.payload)
                }
            },
            (Some(kind), None) => {
                if f.alternate() {
                    write!(f, "{} -> {:#}", kind, self.payload)
                } else {
                    write!(f, "{} -> {}", kind, self.payload)
                }
            },
            (None, Some(message)) => {
                if f.alternate() {
                    write!(f, "{} -> {:#}", message, self.payload)
                } else {
                    write!(f, "{} -> {}", message, self.payload)
                }
            },
            (None, None) => {
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
        match serialize_to_json(&self) {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("Payload<T> serialization error {:?}", err);
                error_json()
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPayload<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    total: usize,
    payload: T,
}

impl<T> ListPayload<T> {
    pub fn new(payload: T) -> Self {
        Self {
            kind: None,
            message: None,
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

    pub fn set_message<M>(&mut self, msg: M) -> ()
    where
        M: Into<String>
    {
        self.message = Some(msg.into());
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>,
    {
        self.message = Some(msg.into());
        self
    }

    pub fn set_total(&mut self, total: usize) -> () {
        self.total = total;
    }

    pub fn with_total(mut self, total: usize) -> Self {
        self.total = total;
        self
    }

    pub fn set_kind<K>(&mut self, kind: K) -> ()
    where
        K: Into<String>
    {
        self.kind = Some(kind.into());
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

impl<T> ListPayload<Vec<T>> {
    pub fn with_vec(vec: Vec<T>) -> Self {
        Self {
            kind: None,
            message: None,
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
            total: slice.len(),
            payload: slice.to_vec(),
        }
    }
}

impl<T> std::fmt::Display for ListPayload<T>
where
    T: std::fmt::Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match (&self.kind, &self.message) {
            (Some(kind), Some(message)) => {
                if f.alternate() {
                    write!(f, "{}: {} -> ({}) {:#}", kind, message, self.total, self.payload)
                } else {
                    write!(f, "{}: {} -> ({}) {}", kind, message, self.total, self.payload)
                }
            },
            (Some(kind), None) => {
                if f.alternate() {
                    write!(f, "{} -> ({}) {:#}", kind, self.total, self.payload)
                } else {
                    write!(f, "{} -> ({}) {}", kind, self.total, self.payload)
                }
            },
            (None, Some(message)) => {
                if f.alternate() {
                    write!(f, "{} -> ({}) {:#}", message, self.total, self.payload)
                } else {
                    write!(f, "{} -> ({}) {}", message, self.total, self.payload)
                }
            },
            (None, None) => {
                if f.alternate() {
                    write!(f, "({}) {:#}", self.total, self.payload)
                } else {
                    write!(f, "({}) {}", self.total, self.payload)
                }
            }
        }
    }
}

impl<T> IntoResponse for ListPayload<T>
where
    T: Serialize
{
    fn into_response(self) -> Response {
        match serialize_to_json(&self) {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("ListPayload<T> serialization error {:?}", err);
                error_json()
            }
        }
    }
}
