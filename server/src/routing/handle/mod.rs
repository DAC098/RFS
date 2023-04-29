use axum::http::{HeaderMap, StatusCode};
use axum::extract::State;
use axum::response::{Response, IntoResponse};
use serde::Serialize;

use crate::net::{self, error};
use crate::state::Shared;

pub mod ping;

#[derive(Serialize)]
pub struct RootContext {}

#[derive(Serialize)]
pub struct RootJson {
    message: String
}

pub async fn get(State(state): State<Shared>, headers: HeaderMap) -> error::Result<impl IntoResponse> {
    if net::html::is_html_accept(&headers)?.is_some() {
        if state.templates().has_template("pages/root") {
            let context = RootContext {};
            let rendered = state.templates().render("pages/root", &context)?;

            return Ok(net::html::html_response(rendered)?
                .into_response());
        }

        let mut working = state.pages().clone();
        working.push("root.html");

        if !working.try_exists()? {
            return Err(error::Error::new()
                .status(StatusCode::NOT_FOUND)
                .kind("NotFound")
                .message("root html was not found"));
        }

        return Ok(net::fs::stream_file(working)
            .await?
            .into_response());
    }

    Ok(net::Json::empty()
        .set_message("no-op")
        .into_response())
}

