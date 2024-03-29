use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error::{self, Context};
use crate::state::ArcShared;
use crate::sec::secrets::Key;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::time;

pub mod version;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
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

    let peppers = state.sec()
        .peppers()
        .inner();
    let mut known_versions;

    {
        let Ok(reader) = peppers.read() else {
            return Err(error::Error::new().context("peppers rwlock poisoned"));
        };

        known_versions = Vec::with_capacity(reader.len());

        for (version, key) in reader.iter() {
            let created = time::utc_to_chrono_datetime(key.created())
                .context("timestamp error for password key")?;

            known_versions.push(rfs_api::sec::secrets::PasswordListItem {
                version: *version,
                created
            });
        }
    }

    Ok(rfs_api::Payload::new(known_versions))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecSecrets,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let wrapper = state.sec().peppers();
    let data = Key::rand_key_data()?;
    let created = time::utc_now().context("failed to create timestamp")?;

    let key = Key::new(data, created);

    {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(error::Error::new().source("peppers rwlock poisoned"));
        };

        writer.update(key);
    }

    wrapper.save()
        .context("failed to save password secret")?;

    Ok(StatusCode::NO_CONTENT)
}
