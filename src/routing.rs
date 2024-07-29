use std::time::Duration;

use axum::Router;
use axum::error_handling::HandleErrorLayer;
use axum::http::{header, StatusCode};
use axum::routing::get;
use axum::response::IntoResponse;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::error::ApiError;
use crate::state::ArcShared;

mod query;
mod layer;
mod serve_file;

mod api;
mod auth;

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

pub fn routes(state: &ArcShared) -> Router {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/api", api::routes())
        .route("/ping", get(ping))
        .fallback(serve_file::handle)
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
