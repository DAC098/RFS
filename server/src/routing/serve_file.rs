use axum::debug_handler;
use axum::extract::State;
use axum::http::{Method, StatusCode, Uri};
use axum::response::IntoResponse;

use crate::net;
use crate::net::error;
use crate::state::ArcShared;

#[debug_handler]
pub async fn handle(
    State(state): State<ArcShared>,
    method: Method,
    uri: Uri
) -> error::Result<impl IntoResponse> {
    if method != Method::GET {
        return Err(error::Error::new()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .kind("MethodNotAllowed"));
    }

    let parts = uri.path().split('/');
    let mut working = state.assets().clone();

    for part in parts {
        if part == ".." || part == "." {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidUri")
                .message("relative paths are not allowed"));
        } else {
            working.push(part);
        }
    }

    if !working.try_exists()? {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("NotFound")
            .message("requested item was not found"));
    }

    if !working.is_file() {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("InvalidFile")
            .message("requested item is not a file"));
    }

    net::fs::stream_file(working).await
}
