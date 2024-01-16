use std::error::Error;
use std::fmt;

use crate::ApiError;

#[derive(Debug)]
pub enum ApiClientError {
    PoisonedLock,
    CookieStore(Box<dyn std::error::Error + Sync + Send>),
    Reqwest(reqwest::Error),
    StdIo(std::io::Error),
}

impl fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiClientError::PoisonedLock => write!(f, "ApiClientError::PoisonedLock"),
            ApiClientError::CookieStore(_) => write!(f, "ApiClientError::CookieStore"),
            ApiClientError::Reqwest(_) => write!(f, "ApiClientError::Reqwest"),
            ApiClientError::StdIo(_) => write!(f, "ApiClientError::StdIo"),
        }
    }
}

impl Error for ApiClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ApiClientError::CookieStore(v) => Some(&**v),
            ApiClientError::Reqwest(v) => Some(v),
            ApiClientError::StdIo(v) => Some(v),
            _ => None
        }
    }
}

#[derive(Debug)]
pub enum RequestError {
    Api(ApiError),
    Reqwest(reqwest::Error)
}

impl RequestError {
    pub fn as_api(self) -> Result<ApiError, reqwest::Error> {
        match self {
            RequestError::Api(v) => Ok(v),
            RequestError::Reqwest(v) => Err(v)
        }
    }
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Api(_) => write!(f, "RequestError::Api"),
            RequestError::Reqwest(_) => write!(f, "RequestError::Reqwest"),
        }
    }
}

impl Error for RequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RequestError::Api(v) => Some(v),
            RequestError::Reqwest(v) => Some(v)
        }
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(err: reqwest::Error) -> Self {
        RequestError::Reqwest(err)
    }
}

impl From<ApiError> for RequestError {
    fn from(err: ApiError) -> Self {
        RequestError::Api(err)
    }
}
