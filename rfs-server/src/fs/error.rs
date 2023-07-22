use crate::net::error;

pub enum StreamError {
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
            StreamError::Io(err) => err.into(),
            StreamError::Axum(err) => err.into(),
        }
    }
}

pub enum BuilderError {
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
            BuilderError::Io(err) => err.into(),
            BuilderError::Pg(err) => err.into(),
            BuilderError::Axum(err) => err.into(),
            BuilderError::Stream(err) => err.into(),
        }
    }
}
