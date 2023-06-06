use axum::http::StatusCode;

pub mod handle;

pub mod serve_file;

pub async fn okay() -> (StatusCode, &'static str) {
    (StatusCode::OK, "pong")
}
