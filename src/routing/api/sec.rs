use axum::Router;
use axum::routing::get;

use crate::state::ArcShared;

mod secrets;
mod roles;

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .route("/secrets/password", get(secrets::password_retrieve)
            .post(secrets::password_create))
        .route("/secrets/password/:version", get(secrets::password_retrieve_version)
            .delete(secrets::password_rotate_deletion))
        .route("/secrets/session", get(secrets::session_retrieve)
            .post(secrets::session_create)
            .delete(secrets::session_delete))
        .route("/roles", get(roles::retrieve)
            .post(roles::create))
        .route("/roles/:role_uid", get(roles::retrieve_id)
            .patch(roles::update_id)
            .delete(roles::delete_id))
        .route("/roles/:role_uid/users", get(roles::retreive_id_users)
            .post(roles::add_id_users)
            .delete(roles::remove_id_users))
        .route("/roles/:role_uid/groups", get(roles::retrieve_id_groups)
            .post(roles::add_id_groups)
            .delete(roles::remove_id_groups))
}
