use http::StatusCode;
use axum_core::response::{Response, IntoResponse};
use serde::{Serialize, Deserialize};
use strum::{AsRefStr as StrumAsRefStr};

use crate::response::{serialize_json, error_json};

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum ApiErrorKind {
    // auth

    PermissionDenied,
    Unauthenticated,
    AlreadyAuthenticated,

    AuthRequired,
    VerifyRequired,

    InvalidPassword,
    InvalidAuthMethod,
    InvalidTotp,
    InvalidTotpHash,

    InvalidSession,
    SessionExpired,
    SessionNotFound,
    SessionUnverified,
    SessionUnauthenticated,

    MechanismNotFound,
    TotpNotFound,
    TotpRecoveryNotFound,

    // sec

    RoleNotFound,
    SecretNotFound,

    // storage

    StorageNotFound,
    DirNotFound,
    NotAbsolutePath,
    NotDirectory,

    // fs

    MaxSize,
    FileNotFound,
    InvalidType,
    NoContentType,
    MimeMismatch,

    // users

    UserNotFound,
    GroupNotFound,

    // tags

    InvalidTags,

    // general

    InternalFailure,
    Timeout,

    AlreadyExists,
    NotFound,

    NoWork,
    NoOp,

    ValidationFailed,
    InvalidData,
    MissingData,

    InvalidProperty,
    InvalidUri,
    InvalidHeaderValue,
    InvalidMimeType,
    InvalidMethod,
    InvalidRequest,
}

impl std::fmt::Display for ApiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&ApiErrorKind> for StatusCode {
    fn from(kind: &ApiErrorKind) -> Self {
        match kind {
            ApiErrorKind::AlreadyAuthenticated |
            ApiErrorKind::InvalidAuthMethod |
            ApiErrorKind::NotAbsolutePath |
            ApiErrorKind::NotDirectory |
            ApiErrorKind::MaxSize |
            ApiErrorKind::InvalidType |
            ApiErrorKind::NoContentType |
            ApiErrorKind::MimeMismatch |
            ApiErrorKind::InvalidTags |
            ApiErrorKind::NoWork |
            ApiErrorKind::NoOp |
            ApiErrorKind::ValidationFailed |
            ApiErrorKind::InvalidData |
            ApiErrorKind::MissingData |
            ApiErrorKind::InvalidProperty |
            ApiErrorKind::InvalidUri |
            ApiErrorKind::InvalidHeaderValue |
            ApiErrorKind::InvalidMimeType |
            ApiErrorKind::InvalidRequest
                => StatusCode::BAD_REQUEST,

            ApiErrorKind::Unauthenticated |
            ApiErrorKind::InvalidSession |
            ApiErrorKind::SessionExpired |
            ApiErrorKind::SessionNotFound |
            ApiErrorKind::SessionUnverified |
            ApiErrorKind::SessionUnauthenticated |
            ApiErrorKind::MechanismNotFound
                => StatusCode::UNAUTHORIZED,

            ApiErrorKind::PermissionDenied |
            ApiErrorKind::AuthRequired |
            ApiErrorKind::VerifyRequired |
            ApiErrorKind::InvalidPassword |
            ApiErrorKind::InvalidTotp |
            ApiErrorKind::InvalidTotpHash
                => StatusCode::FORBIDDEN,

            ApiErrorKind::TotpNotFound |
            ApiErrorKind::TotpRecoveryNotFound |
            ApiErrorKind::RoleNotFound |
            ApiErrorKind::SecretNotFound |
            ApiErrorKind::StorageNotFound |
            ApiErrorKind::DirNotFound |
            ApiErrorKind::FileNotFound |
            ApiErrorKind::UserNotFound |
            ApiErrorKind::GroupNotFound |
            ApiErrorKind::NotFound
                => StatusCode::NOT_FOUND,

            ApiErrorKind::InvalidMethod
                => StatusCode::METHOD_NOT_ALLOWED,

            ApiErrorKind::Timeout
                => StatusCode::REQUEST_TIMEOUT,

            ApiErrorKind::AlreadyExists
                => StatusCode::CONFLICT,

            ApiErrorKind::InternalFailure
                => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Detail {
    Keys(Vec<String>),
}

impl Detail {
    pub fn with_key(key: impl Into<String>) -> Self {
        Detail::Keys(vec![key.into()])
    }
}

impl From<&str> for Detail {
    fn from(key: &str) -> Detail {
        Detail::Keys(vec![key.to_owned()])
    }
}

impl From<String> for Detail {
    fn from(key: String) -> Detail {
        Detail::Keys(vec![key])
    }
}

impl From<Vec<String>> for Detail {
    fn from(keys: Vec<String>) -> Detail {
        Detail::Keys(keys)
    }
}

impl From<&[&str]> for Detail {
    fn from(keys: &[&str]) -> Detail {
        Detail::Keys(keys.iter().map(|v| (*v).into()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for Detail {
    fn from(keys: [&str; N]) -> Detail {
        Detail::Keys(keys.iter().map(|v| (*v).into()).collect())
    }
}

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Detail::Keys(list) => {
                let mut iter = list.iter();

                write!(f, "Detail::Keys(")?;

                if let Some(first) = iter.next() {
                    write!(f, "{}", first)?;

                    while let Some(key) = iter.next() {
                        write!(f, ",{}", key)?;
                    }
                }

                write!(f, ")")?;
            },
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    kind: ApiErrorKind,
    detail: Option<Detail>,
    msg: Option<String>,
}

impl ApiError {
    pub fn new() -> Self {
        ApiError {
            kind: ApiErrorKind::InternalFailure,
            detail: None,
            msg: None
        }
    }

    pub fn with_kind<K>(mut self, kind: K) -> Self
    where
        K: Into<ApiErrorKind>
    {
        self.kind = kind.into();
        self
    }

    pub fn with_detail(mut self, detail: Detail) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn with_message<M>(mut self, msg: M) -> Self
    where
        M: Into<String>
    {
        self.msg = Some(msg.into());
        self
    }

    pub fn kind(&self) -> &ApiErrorKind {
        &self.kind
    }

    pub fn detail(&self) -> Option<&Detail> {
        self.detail.as_ref()
    }

    pub fn message(&self) -> Option<&str> {
        self.msg.as_ref().map(|v| v.as_str())
    }
}

impl std::default::Default for ApiError {
    fn default() -> Self {
        ApiError::new()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;

        if let Some(detail) = &self.detail {
            write!(f, ": {}", detail)?;
        }

        if let Some(msg) = &self.msg {
            write!(f, ": {}", msg)?;
        }

        Ok(())
    }
}

impl From<ApiErrorKind> for ApiError
{
    fn from(kind: ApiErrorKind) -> Self {
        ApiError {
            kind,
            detail: None,
            msg: None
        }
    }
}

impl<D> From<(ApiErrorKind, D)> for ApiError
where
    D: Into<Detail>
{
    fn from((kind, detail): (ApiErrorKind, D)) -> Self {
        ApiError {
            kind,
            detail: Some(detail.into()),
            msg: None
        }
    }
}

impl<D, M> From<(ApiErrorKind, D, M)> for ApiError
where
    D: Into<Detail>,
    M: Into<String>
{
    fn from((kind, detail, msg): (ApiErrorKind, D, M)) -> Self {
        ApiError {
            kind,
            detail: Some(detail.into()),
            msg: Some(msg.into())
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = (&self.kind).into();

        match serialize_json(status, &self) {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("ApiError serialization error {:?}", err);
                error_json()
            }
        }
    }
}
