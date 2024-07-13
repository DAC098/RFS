use axum::Router;
use axum::error_handling::HandleErrorLayer;
use tower::ServiceBuilder;

use crate::state::ArcShared;
use crate::error::ApiError;
use crate::error::api::ApiErrorKind;

mod auth;
mod sec;
mod user;
mod fs;

async fn not_found() -> ApiError {
    ApiError::from(ApiErrorKind::NotFound)
}

async fn handle_error<E>(error: E) -> ApiError
where
    E: Into<ApiError>
{
    let error = error.into();

    if let Some(err) = std::error::Error::source(&error) {
        tracing::error!("unhandled error when processing request: {err:#?}");
    }

    error
}

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/sec", sec::routes())
        .nest("/user", user::routes())
        .nest("/fs", fs::routes())
        .fallback(not_found)
        .layer(ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_error)))
}
