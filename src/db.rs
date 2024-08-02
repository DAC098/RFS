use std::future::Future;
use std::pin::Pin;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

pub use deadpool_postgres::Object;
//pub use deadpool_postgres::GenericClient;
//pub use tokio_postgres::Error;

use crate::error::ApiError;
use crate::state::ArcShared;

pub struct Conn(pub Object);

impl FromRequestParts<ArcShared> for Conn {
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        _parts: &'life0 mut Parts,
        state: &'life1 ArcShared
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait
    {
        let fut = state.pool().get();

        Box::pin(async move {
            match fut.await {
                Ok(obj) => Ok(Conn(obj)),
                Err(err) => Err(ApiError::from(err)
                    .context("failed to retrieve database connection"))
            }
        })
    }
}
