use axum::Router;
use axum::routing::{post, delete};

use crate::state::ArcShared;

mod session;

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .route("/session/request", post(session::request))
        .route("/session/submit", post(session::submit))
        .route("/session/verify", post(session::verify))
        .route("/session/drop", delete(session::drop))
}
