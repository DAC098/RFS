use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::net::{self, error};
use crate::state::ArcShared;

pub mod totp;

#[derive(Serialize)]
pub struct AuthContext {}

pub async fn get(
    State(state): State<ArcShared>,
    headers: HeaderMap
) -> error::Result<impl IntoResponse> {
    if net::html::is_html_accept(&headers)?.is_some() {
        if state.templates().has_template("pages/auth") {
            let context = AuthContext {};
            let rendered = state.templates().render("pages/auth", &context)?;

            return Ok(net::html::html_response(rendered)?
                .into_response());
        }

        return Ok(net::fs::response_file(
            "auth html",
            state.pages().join("auth.html")
        ).await?.into_response())
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}
