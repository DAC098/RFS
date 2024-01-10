use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;

use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;

use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::time;

#[derive(Deserialize)]
pub struct PathParams {
    version: u64
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { version }): Path<PathParams>
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecSecrets,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let peppers = state.sec().peppers().inner();

    let (data, created) = {
        let Ok(reader) = peppers.read() else {
            return Err(error::Error::new().source("peppers rwlock poisoned"));
        };

        let Some(found) = reader.get(&version) else {
            return Err(error::Error::api(error::ApiErrorKind::SecretNotFound));
        };

        found.clone().into_tuple()
    };

    let conn = state.pool().get().await?;

    let count = conn.execute(
        "select auth_password.user_id from auth_password where auth_password.version = $1",
        &[&(version as i64)]
    ).await?;

    let Some(created) = time::utc_to_chrono_datetime(&created) else {
        return Err(error::Error::new().source("timetamp error for password key"));
    };

    Ok(rfs_api::Payload::new(rfs_api::sec::secrets::PasswordVersion {
        version,
        created,
        data: data.into(),
        in_use: count
    }))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { version }): Path<PathParams>
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecSecrets,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let wrapper = state.sec().peppers();

    let _found = {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(error::Error::new().source("peppers rwlock poisoned"));
        };

        let Some(found) = writer.remove(&version) else {
            return Err(error::Error::api(error::ApiErrorKind::SecretNotFound));
        };

        found
    };

    if let Err(err) = wrapper.save() {
        return Err(error::Error::new().source(err));
    }

    Ok(StatusCode::NO_CONTENT)
}
