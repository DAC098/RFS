use axum::debug_handler;
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Uri, StatusCode};
use axum::response::Response;
use tokio::fs::OpenOptions;
use tokio_util::io::ReaderStream;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{ApiErrorKind, Context};
use crate::path;
use crate::state::ArcShared;

#[debug_handler]
pub async fn handle(
    State(state): State<ArcShared>,
    method: Method,
    uri: Uri
) -> ApiResult<Response<Body>> {
    if method != Method::GET {
        return Err(ApiError::from(ApiErrorKind::InvalidMethod));
    }

    let parts = uri.path().split('/');
    let mut working = state.assets().clone();

    for part in parts {
        if part == ".." || part == "." {
            return Err(ApiError::from(ApiErrorKind::InvalidUri));
        } else {
            working.push(part);
        }
    }

    let mime = path::mime_from_ext(working.extension());

    let metadata = path::metadata(&working)
        .context("error when retrieving metadata for file")?
        .kind(ApiErrorKind::NotFound)?;

    if !metadata.is_file() {
        return Err(ApiError::from(ApiErrorKind::InvalidRequest));
    }

    let file = OpenOptions::new()
        .read(true)
        .open(working)
        .await
        .context("failed opening file for reading")?;

    let stream = ReaderStream::new(file);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.to_string())
        .header(header::CONTENT_LENGTH, metadata.len())
        .body(Body::from_stream(stream))?)
}
