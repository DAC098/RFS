use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use lib::{ids, models};

use crate::net;
use crate::net::error;
use crate::state::ArcShared;

#[derive(Deserialize)]
pub struct PathParams {
    storage_id: ids::StorageId,
    fs_id: ids::FSId,
}

pub async fn get(
    State(state): State<ArcShared>, 
    Path(PathParams { storage_id, fs_id }): Path<PathParams>
) -> error::Result<impl IntoResponse> {
    Ok(net::Json::empty())
}

pub async fn post(
    State(state): State<ArcShared>,
    Path(PathParams { storage_id, fs_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    Ok(net::Json::empty())
}

pub async fn put(
    State(state): State<ArcShared>,
    Path(PathParams { storage_id, fs_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    Ok(net::Json::empty())
}

pub async fn delete(
    State(state): State<ArcShared>,
    Path(PathParams { storage_id, fs_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    Ok(net::Json::empty())
}
