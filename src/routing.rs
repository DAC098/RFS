use std::time::Duration;

use axum::{debug_handler, Router};
use axum::extract::State;
use axum::error_handling::HandleErrorLayer;
use axum::http::{header, Method, Uri, StatusCode};
use axum::routing::get;
use axum::response::IntoResponse;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::error::{ApiError, ApiResult};
use crate::error::api::ApiErrorKind;
use crate::net;
use crate::state::ArcShared;

mod query;

mod api;

mod layer;
mod handle;

async fn okay() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

async fn ping() -> (StatusCode, &'static str) {
    (StatusCode::OK, "pong")
}

async fn handle_error<E>(error: E) -> impl IntoResponse
where
    E: Into<ApiError>
{
    let error = error.into();

    if let Some(err) = std::error::Error::source(&error) {
        tracing::error!("unhandled error when processing request: {err:#?}");
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(header::CONTENT_TYPE, "text/plain")],
        "internal server error"
    )
}

#[debug_handler]
async fn serve_file(
    State(state): State<ArcShared>,
    method: Method,
    uri: Uri
) -> ApiResult<impl IntoResponse> {
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

    if !working.try_exists()? {
        return Err(ApiError::api(ApiErrorKind::NotFound));
    }

    if !working.is_file() {
        return Err(ApiError::api(ApiErrorKind::InvalidRequest));
    }

    net::fs::stream_file(working).await
}

pub fn routes(state: &ArcShared) -> Router {
    Router::new()
        .nest("/api", api::routes())
        .route(
            "/",
            get(handle::get)
        )
        .route(
            "/fs/storage",
            get(handle::fs::storage::get)
                .post(handle::fs::storage::post)
        )
        .route(
            "/fs/storage/:storage_id",
            get(handle::fs::storage::storage_id::get)
                .put(handle::fs::storage::storage_id::put)
                .delete(handle::fs::storage::storage_id::delete)
        )
        .route(
            "/fs/roots",
            get(handle::fs::roots::get)
        )
        .route(
            "/fs/:fs_id",
            get(handle::fs::fs_id::get)
                .post(handle::fs::fs_id::post)
                .put(handle::fs::fs_id::put)
                .patch(handle::fs::fs_id::patch)
                .delete(handle::fs::fs_id::delete)
        )
        .route(
            "/fs/:fs_id/contents",
            get(handle::fs::fs_id::contents::get)
        )
        .route(
            "/fs/:fs_id/dl",
            get(handle::fs::fs_id::dl::get)
        )
        .route("/ping", get(ping))
        .fallback(serve_file)
        .layer(ServiceBuilder::new()
            .layer(layer::RIDLayer::new())
            .layer(TraceLayer::new_for_http()
                .make_span_with(layer::make_span_with)
                .on_request(layer::on_request)
                .on_response(layer::on_response)
                .on_failure(layer::on_failure))
            .layer(HandleErrorLayer::new(handle_error))
            .layer(layer::TimeoutLayer::new(Duration::new(90, 0))))
        .with_state(state.clone())
}
