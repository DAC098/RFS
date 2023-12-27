use rfs_lib::schema;


use axum::extract::State;
use axum::response::IntoResponse;


use crate::net::{self, error};
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
        initiator.user().id(),
        permission::Scope::SecSecrets,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let peppers = state.sec().peppers().inner();
    let mut known_versions;

    {
        let Ok(reader) = peppers.read() else {
            return Err(error::Error::new().source("peppers rwlock poisoned"));
        };

        known_versions = Vec::with_capacity(reader.len());

        for (version, key) in reader.iter() {
            let Some(created) = time::utc_to_chrono_datetime(key.created()) else {
                return Err(error::Error::new().source("timestamp error for password key"));
            };

            known_versions.push(schema::sec::PasswordListItem {
                version: *version,
                created
            });
        }
    }

    Ok(net::Json::new(rfs_lib::json::ListWrapper::with_vec(known_versions)))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::SecSecrets,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let wrapper = state.sec().peppers();
    let data = Key::rand_key_data()?;
    let Some(created) = time::utc_now() else {
        return Err(error::Error::new().source("failed to create timestamp"));
    };

    let key = Key::new(data, created);

    {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(error::Error::new().source("peppers rwlock poisoned"));
        };

        writer.update(key);
    }

    if let Err(err) = wrapper.save() {
        return Err(error::Error::new().source(err));
    }

    Ok(net::Json::empty())
}
