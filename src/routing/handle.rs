use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::net::{self, error};
use crate::state::ArcShared;

pub mod ping;
pub mod auth;
pub mod sec;
pub mod storage;
pub mod fs;
pub mod user;

#[derive(Serialize)]
pub struct RootContext {}

#[derive(Serialize)]
pub struct RootJson {
    message: String
}

pub async fn get(
    State(state): State<ArcShared>, 
    headers: HeaderMap
) -> error::Result<impl IntoResponse> {
    if net::html::is_html_accept(&headers)?.is_some() {
        if state.templates().has_template("pages/root") {
            let context = RootContext {};
            let rendered = state.templates().render("pages/root", &context)?;

            return Ok(net::html::html_response(rendered)?
                .into_response());
        }

        return Ok(net::fs::response_file(
            "root html",
            state.pages().join("root.html")
        ).await?.into_response())
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

