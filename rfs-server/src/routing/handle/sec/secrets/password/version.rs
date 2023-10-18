use rfs_lib::{schema, actions};
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use chrono::{Utc, DateTime};
use serde::Deserialize;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::secrets::Key;
use crate::time;

#[derive(Deserialize)]
pub struct PathParams {
    version: u64
}

pub async fn get(
    State(state): State<ArcShared>,
    Path(PathParams { version }): Path<PathParams>
) -> error::Result<impl IntoResponse> {
    let peppers = state.sec().peppers().inner();

    let (data, created) = {
        let reader = peppers.read()
            .map_err(|_| error::Error::new().source("peppers rwlock poisoned"))?;

        let Some(found) = reader.get(&version) else {
            return Err(error::Error::new()
                .status(StatusCode::NOT_FOUND)
                .kind("SecretNotFound")
                .message("requested secret version was not found"));
        };

        found.clone().into_tuple()
    };

    let conn = state.pool().get().await?;

    let count = conn.execute(
        "select auth_password.user_id from auth_password where auth_password.version = $1",
        &[&(version as i64)]
    ).await?;

    let created = time::utc_to_chrono_datetime(&created)
        .ok_or(error::Error::new()
            .kind("TimestampError")
            .message("failed to create timestamp for password key"))?;

    let rtn = schema::sec::PasswordVersion {
        version,
        created,
        data: data.into(),
        in_use: count
    };

    Ok(net::Json::new(rtn))
}

pub async fn delete(
    State(state): State<ArcShared>,
    Path(PathParams { version }): Path<PathParams>
) -> error::Result<impl IntoResponse> {
    let wrapper = state.sec().peppers();

    let found = {
        let mut writer = wrapper.inner().write()
            .map_err(|_| error::Error::new().source("peppers rwlock poisoned"))?;

        let Some(found) = writer.remove(&version) else {
            return Err(error::Error::new()
                .status(StatusCode::NOT_FOUND)
                .kind("SecretNotFound")
                .message("requested secret version was not found"));
        };

        found
    };

    wrapper.save()
        .map_err(|e| error::Error::new()
            .kind("FailedSavingPeppers")
            .message("failed to save updated peppers to file")
            .source(e))?;

    Ok(net::Json::empty())
}
