use axum::http::StatusCode;

pub async fn get() -> (StatusCode, &'static str) {
    (StatusCode::OK, "pong")
}
