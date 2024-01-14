use crate::ApiError;

pub enum ApiClientError {
    PoisonedLock,
    CookieStore(Box<dyn std::error::Error + Sync + Send>),
    Reqwest(reqwest::Error),
    StdIo(std::io::Error),
}

pub enum RequestError {
    Api(ApiError),
    Reqwest(reqwest::Error)
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
