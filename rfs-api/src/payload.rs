use rfs_lib::query::{Limit, Offset};

use http::StatusCode;
use axum_core::response::{Response, IntoResponse};
use serde::{Serialize, Deserialize};

use crate::response::{serialize_json, error_json};

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload<T> {
    #[serde(default, skip_serializing_if = "Option::is_none", flatten)]
    pagination: Option<Pagination>,

    payload: T
}

impl<T> Payload<T> {
    pub fn new(payload: T) -> Self {
        Self {
            pagination: None,
            payload
        }
    }

    pub fn pagination(&self) -> Option<&Pagination> {
        self.pagination.as_ref()
    }

    pub fn set_pagination<P>(mut self, p: P) -> Self
    where
        P: Into<Pagination>
    {
        self.pagination = Some(p.into());
        self
    }

    pub fn with_pagination<P>(&mut self, p: P) -> &mut Self
    where
        P: Into<Pagination>
    {
        self.pagination = Some(p.into());
        self
    }

    pub fn payload(&self) -> &T {
        &self.payload
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
            pagination: self.pagination,
            payload
        }
    }

    pub fn into_payload(self) -> T {
        self.payload
    }

    pub fn into_tuple(self) -> PayloadTuple<T> {
        (self.pagination, self.payload)
    }
}

impl<T> std::fmt::Display for Payload<T>
where
    T: std::fmt::Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if f.alternate() {
            write!(f, "{:#}", self.payload)
        } else {
            write!(f, "{}", self.payload)
        }
    }
}

impl<P, T> From<(P, T)> for Payload<T>
where
    P: Into<Pagination>
{
    fn from((p, t): (P, T)) -> Self {
        Payload {
            pagination: Some(p.into()),
            payload: t
        }
    }
}

pub type PayloadTuple<T> = (Option<Pagination>, T);

impl<T> From<Payload<T>> for PayloadTuple<T> {
    fn from(p: Payload<T>) -> PayloadTuple<T> {
        (p.pagination, p.payload)
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    limit: Limit,
    offset: Option<Offset>
}

impl Pagination {
    pub fn new() -> Self {
        Self {
            limit: Limit::Small,
            offset: None,
        }
    }

    pub fn limit(&self) -> &Limit {
        &self.limit
    }

    pub fn with_limit(mut self, limit: Limit) -> Self {
        self.limit = limit;
        self
    }

    pub fn set_limit(&mut self, limit: Limit) -> &mut Self {
        self.limit = limit;
        self
    }

    pub fn offset(&self) -> Option<&Offset> {
        self.offset.as_ref()
    }

    pub fn with_offset(mut self, offset: Offset) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn set_offset(&mut self, offset: Offset) -> &mut Self {
        self.offset = Some(offset);
        self
    }
}

impl std::default::Default for Pagination {
    fn default() -> Self {
        Pagination::new()
    }
}

impl From<(Limit, Offset)> for Pagination {
    fn from(v: (Limit, Offset)) -> Self {
        Pagination {
            limit: v.0,
            offset: Some(v.1),
        }
    }
}

impl From<Limit> for Pagination {
    fn from(limit: Limit) -> Self {
        Pagination {
            limit,
            offset: None,
        }
    }
}

impl From<&Limit> for Pagination {
    fn from(limit: &Limit) -> Self {
        Pagination {
            limit: limit.clone(),
            offset: None,
        }
    }
}

impl From<Pagination> for (Limit, Option<Offset>) {
    fn from(p: Pagination) -> Self {
        (p.limit, p.offset)
    }
}

impl From<Pagination> for Limit {
    fn from(p: Pagination) -> Self {
        p.limit
    }
}
