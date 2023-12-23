use http::StatusCode;
use serde::{Serialize, Deserialize};
use strum::{AsRefStr as StrumAsRefStr};

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum AuthKind {
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

    MechanismNotFound
}

impl std::fmt::Display for AuthKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&AuthKind> for StatusCode {
    fn from(kind: &AuthKind) -> Self {
        match kind {
            AuthKind::PermissionDenied |
            AuthKind::AuthRequired |
            AuthKind::VerifyRequired |
            AuthKind::InvalidPassword |
            AuthKind::InvalidTotp |
            AuthKind::InvalidTotpHash => StatusCode::FORBIDDEN,
            AuthKind::Unauthenticated |
            AuthKind::InvalidSession |
            AuthKind::SessionExpired |
            AuthKind::SessionNotFound |
            AuthKind::SessionUnverified |
            AuthKind::SessionUnauthenticated |
            AuthKind::MechanismNotFound => StatusCode::UNAUTHORIZED,
            AuthKind::AlreadyAuthenticated |
            AuthKind::VerifyRequired |
            AuthKind::InvalidAuthMethod => StatusCode::BAD_REQUEST,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum SecKind {
    RoleNotFound,
    SecretNotFound,
}

impl std::fmt::Display for SecKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&SecKind> for StatusCode {
    fn from(kind: &SecKind) -> Self {
        match kind {
            SecKind::RoleNotFound |
            SecKind::SecretNotFound => StatusCode::NOT_FOUND
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum StorageKind {
    NotFound,
    DirNotFound,

    NotAbsolutePath,
    NotDirectory,
}

impl std::fmt::Display for StorageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&StorageKind> for StatusCode {
    fn from(kind: &StorageKind) -> Self {
        match kind {
            StorageKind::NotFound |
            StorageKind::DirNotFound => StatusCode::NOT_FOUND,
            StorageKind::NotAbsolutePath |
            StorageKind::NotDirectory => StatusCode::BAD_REQUEST,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum FsKind {
    MaxSize,
    NotFound,
    InvalidType,
    NoContentType,
    MimeMismatch,
}

impl std::fmt::Display for FsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&FsKind> for StatusCode {
    fn from(kind: &FsKind) -> Self {
        match kind {
            FsKind::MaxSize |
            FsKind::InvalidType |
            FsKind::NoContentType |
            FsKind::MimeMismatch => StatusCode::BAD_REQUEST,
            FsKind::NotFound => StatusCode::NOT_FOUND,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum UserKind {
    NotFound,
    GroupNotFound,
}

impl std::fmt::Display for UserKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&UserKind> for StatusCode {
    fn from(kind: &UserKind) -> Self {
        match kind {
            UserKind::NotFound |
            UserKind::GroupNotFound => StatusCode::NOT_FOUND,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum TagKind {
    InvalidTags
}

impl std::fmt::Display for TagKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&TagKind> for StatusCode {
    fn from(kind: &TagKind) -> Self {
        match kind {
            TagKind::InvalidTags => StatusCode::BAD_REQUEST
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    StrumAsRefStr,
    Serialize, Deserialize
)]
pub enum GeneralKind {
    InternalFailure,
    Timeout,

    AlreadyExists,
    NotFound,

    NoWork,
    Noop,

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

impl std::fmt::Display for GeneralKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_ref(), f)
    }
}

impl From<&GeneralKind> for StatusCode {
    fn from(kind: &GeneralKind) -> StatusCode {
        match kind {
            GeneralKind::InternalFailure => StatusCode::INTERNAL_SERVER_ERROR,
            GeneralKind::Timeout => StatusCode::REQUEST_TIMEOUT,
            GeneralKind::AlreadyExists => StatusCode::CONFLICT,
            GeneralKind::NotFound => StatusCode::NOT_FOUND,
            GeneralKind::NoWork |
            GeneralKind::Noop |
            GeneralKind::ValidationFailed |
            GeneralKind::InvalidData |
            GeneralKind::MissingData |
            GeneralKind::InvalidProperty |
            GeneralKind::InvalidUri |
            GeneralKind::InvalidHeaderValue |
            GeneralKind::InvalidMimeType |
            GeneralKind::InvalidRequest => StatusCode::BAD_REQUEST,
            GeneralKind::InvalidMethod => StatusCode::METHOD_NOT_ALLOWED,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    Serialize, Deserialize
)]
pub enum ApiErrorKind {
    General(GeneralKind),
    Auth(AuthKind),
    Sec(SecKind),
    Storage(StorageKind),
    Fs(FsKind),
    Tag(TagKind),
    User(UserKind),
}

impl std::fmt::Display for ApiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiErrorKind::General(v) => std::fmt::Display::fmt(v, f),
            ApiErrorKind::Auth(v) => std::fmt::Display::fmt(v, f),
            ApiErrorKind::Sec(v) => std::fmt::Display::fmt(v, f),
            ApiErrorKind::Storage(v) => std::fmt::Display::fmt(v, f),
            ApiErrorKind::Fs(v) => std::fmt::Display::fmt(v, f),
            ApiErrorKind::Tag(v) => std::fmt::Display::fmt(v, f),
            ApiErrorKind::User(v) => std::fmt::Display::fmt(v, f),
        }
    }
}

impl From<GeneralKind> for ApiErrorKind {
    fn from(v: GeneralKind) -> Self {
        ApiErrorKind::General(v)
    }
}

impl From<AuthKind> for ApiErrorKind {
    fn from(v: AuthKind) -> Self {
        ApiErrorKind::Auth(v)
    }
}

impl From<SecKind> for ApiErrorKind {
    fn from(v: SecKind) -> Self {
        ApiErrorKind::Sec(v)
    }
}

impl From<StorageKind> for ApiErrorKind {
    fn from(v: StorageKind) -> Self {
        ApiErrorKind::Storage(v)
    }
}

impl From<FsKind> for ApiErrorKind {
    fn from(v: FsKind) -> Self {
        ApiErrorKind::Fs(v)
    }
}

impl From<TagKind> for ApiErrorKind {
    fn from(v: TagKind) -> Self {
        ApiErrorKind::Tag(v)
    }
}

impl From<UserKind> for ApiErrorKind {
    fn from(v: UserKind) -> Self {
        ApiErrorKind::User(v)
    }
}

impl From<&ApiErrorKind> for StatusCode {
    fn from(kind: &ApiErrorKind) -> Self {
        match kind {
            ApiErrorKind::General(v) => v.into(),
            ApiErrorKind::Auth(v) => v.into(),
            ApiErrorKind::Sec(v) => v.into(),
            ApiErrorKind::Storage(v) => v.into(),
            ApiErrorKind::Fs(v) => v.into(),
            ApiErrorKind::Tag(v) => v.into(),
            ApiErrorKind::User(v) => v.into(),
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

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Detail::Keys(list) => {
                let mut iter = list.iter();

                if let Some(first) = iter.next() {
                    write!(f, "{}", first)?;

                    while let Some(key) = iter.next() {
                        write!(f, ",{}", key)?;
                    }
                }
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
            kind: ApiErrorKind::General(GeneralKind::InternalFailure),
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

impl<K> From<K> for ApiError
where
    K: Into<ApiErrorKind>
{
    fn from(kind: K) -> Self {
        ApiError {
            kind: kind.into(),
            detail: None,
            msg: None
        }
    }
}

impl<K,M> From<(K, M)> for ApiError
where
    K: Into<ApiErrorKind>,
    M: Into<String>,
{
    fn from((kind, msg): (K, M)) -> Self {
        ApiError {
            kind: kind.into(),
            detail: None,
            msg: Some(msg.into())
        }
    }
}

impl<K> From<(K, Detail)> for ApiError
where
    K: Into<ApiErrorKind>
{
    fn from((kind, detail): (K, Detail)) -> Self {
        ApiError {
            kind: kind.into(),
            detail: Some(detail),
            msg: None
        }
    }
}

impl<K,M> From<(K, Detail, M)> for ApiError
where
    K: Into<ApiErrorKind>,
    M: Into<String>
{
    fn from((kind, detail, msg): (K, Detail, M)) -> Self {
        ApiError {
            kind: kind.into(),
            detail: Some(detail),
            msg: Some(msg.into())
        }
    }
}
