use axum::http::StatusCode;
use tokio_postgres::Error as PgError;

use crate::net::error;

pub enum BuilderError {
    NameExists,
    PathNotDirectory,
    Pg(PgError),
    Io(std::io::Error),
}

impl From<PgError> for BuilderError {
    fn from(err: PgError) -> Self {
        BuilderError::Pg(err)
    }
}

impl From<std::io::Error> for BuilderError {
    fn from(err: std::io::Error) -> Self {
        BuilderError::Io(err)
    }
}

impl From<BuilderError> for error::Error {
    fn from(err: BuilderError) -> Self {
        match err {
            BuilderError::NameExists => error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("NameExists")
                .message("the requested name already exists"),
            BuilderError::PathNotDirectory => error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("PathNotDirectory")
                .message("the requested path is not a directory"),
            BuilderError::Pg(err) => err.into(),
            BuilderError::Io(err) => err.into()
        }
    }
}
