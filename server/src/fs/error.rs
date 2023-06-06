use axum::http::StatusCode;

use crate::net::error;

pub enum StreamError {
    MaxFileSize,
    Axum(axum::Error),
    Io(std::io::Error),
}

impl From<axum::Error> for StreamError {
    fn from(err: axum::Error) -> Self {
        StreamError::Axum(err)
    }
}

impl From<std::io::Error> for StreamError {
    fn from(err: std::io::Error) -> Self {
        StreamError::Io(err)
    }
}

impl From<StreamError> for error::Error {
    fn from(err: StreamError) -> Self {
        match err {
            StreamError::MaxFileSize => error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("MaxFileSize")
                .message("provided file is too large"),
            StreamError::Io(err) => err.into(),
            StreamError::Axum(err) => err.into(),
        }
    }
}

pub enum BuilderError {
    BasenameExists,
    BasenameGenFailed,
    Io(std::io::Error),
    Pg(tokio_postgres::Error),
    Axum(axum::Error),
    Stream(StreamError),
}

impl From<std::io::Error> for BuilderError {
    fn from(err: std::io::Error) -> Self {
        BuilderError::Io(err)
    }
}

impl From<tokio_postgres::Error> for BuilderError {
    fn from(err: tokio_postgres::Error) -> Self {
        BuilderError::Pg(err)
    }
}

impl From<axum::Error> for BuilderError {
    fn from(err: axum::Error) -> Self {
        BuilderError::Axum(err)
    }
}

impl From<StreamError> for BuilderError {
    fn from(err: StreamError) -> Self {
        BuilderError::Stream(err)
    }
}

impl From<BuilderError> for error::Error {
    fn from(err: BuilderError) -> Self {
        match err {
            BuilderError::BasenameExists => error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("BasenameExists")
                .message("basename already exists in this directory"),
            BuilderError::BasenameGenFailed => error::Error::new()
                .kind("BasenameGenFailed")
                .message("failed to generate basename"),
            BuilderError::Io(err) => err.into(),
            BuilderError::Pg(err) => err.into(),
            BuilderError::Axum(err) => err.into(),
            BuilderError::Stream(err) => err.into(),
        }
    }
}
