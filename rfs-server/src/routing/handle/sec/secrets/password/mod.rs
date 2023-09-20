use rfs_lib::schema;
use rfs_lib::actions;
use axum::http::{HeaderMap, StatusCode};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;

pub async fn get(
    State(state): State<ArcShared>,
) -> error::Result<impl IntoResponse> {
    Ok(net::Json::empty())
}

pub async fn post(
    State(state): State<ArcShared>,
) -> error::Result<impl IntoResponse> {
    let result = state.peppers().create(
    Ok(net::Json::empty())
}
