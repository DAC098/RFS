use axum::Router;
use axum::routing::{get, post, delete};

use crate::state::ArcShared;

mod session;
mod password;
mod totp;

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .route("/session/request", post(session::request))
        .route("/session/submit", post(session::submit))
        .route("/session/verify", post(session::verify))
        .route("/session/drop", delete(session::drop))
        .route("/password", post(password::update))
        .route("/totp", get(totp::retrieve)
            .post(totp::create)
            .patch(totp::update)
            .delete(totp::delete))
        .route("/totp/recovery", get(totp::retrieve_recovery)
            .post(totp::create_recovery))
        .route("/totp/recovery/:key_id", get(totp::retrieve_recovery_key)
            .patch(totp::update_recovery_key)
            .delete(totp::delete_recovery_key))
}
