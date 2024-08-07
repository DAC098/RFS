use axum::Router;
use axum::error_handling::HandleErrorLayer;
use tower::ServiceBuilder;

use crate::state::ArcShared;
use crate::error::ApiError;
use crate::error::api::ApiErrorKind;

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
    error.into()
}

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .nest("/sec", sec::routes())
        .nest("/user", user::routes())
        .nest("/fs", fs::routes())
        .fallback(not_found)
        .layer(ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_error)))
}
