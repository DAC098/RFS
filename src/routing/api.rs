use axum::Router;
use axum::error_handling::HandleErrorLayer;
use tower::ServiceBuilder;

use crate::state::ArcShared;
use crate::error;

mod auth;

async fn not_found() -> error::ApiError {
    error::ApiError::from(error::api::ApiErrorKind::NotFound)
}

async fn handle_error<E>(error: E) -> error::ApiError
where
    E: Into<error::ApiError>
{
    let error = error.into();

    if let Some(err) = std::error::Error::source(&error) {
        tracing::error!("unhandled error when prcessing request: {err:#?}");
    }

    error
}

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .nest("/auth", auth::routes())
        .fallback(not_found)
        .layer(ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_error)))
}
