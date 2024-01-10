use axum::http::StatusCode;
use axum::extract::{Query, State};
use axum::response::IntoResponse;

use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::secrets::Key;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::time;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecSecrets,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let session_keys = state.sec().session_info().keys().inner();
    let mut known_keys;

    {
        let Ok(reader) = session_keys.read() else {
            return Err(error::Error::new().source("session keys rwlock poisoned"));
        };

        known_keys = Vec::with_capacity(reader.stored());

        for key in reader.iter() {
            let Some(created) = time::utc_to_chrono_datetime(key.created()) else {
                return Err(error::Error::new().source("timestamp error for session key"));
            };

            known_keys.push(rfs_api::sec::secrets::SessionListItem { created });
        }
    }

    Ok(rfs_api::Payload::new(known_keys))
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
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let wrapper = state.sec().session_info().keys();
    let data = Key::rand_key_data()?;
    let Some(created) = time::utc_now() else {
        return Err(error::Error::new().source("timestamp error for session key"));
    };

    let key = Key::new(data, created);

    {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(error::Error::new().source("session keys rwlock poisoned"));
        };

        writer.push(key);
    }

    if let Err(err) = wrapper.save() {
        return Err(error::Error::new().source(err));
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    amount: Option<usize>
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Query(query): Query<DeleteQuery>,
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

    let wrapper = state.sec().session_info().keys();

    let Some(mut amount) = query.amount else {
        return Ok(StatusCode::NO_CONTENT);
    };

    if amount == 0 {
        return Ok(StatusCode::NO_CONTENT);
    }

    {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(error::Error::new().source("session keys rwlock poisoned"));
        };

        while amount > 0 {
            if let None = writer.pop() {
                break;
            }

            amount -= 1;
        }
    }

    if let Err(err) = wrapper.save() {
        return Err(error::Error::new().source(err));
    }

    Ok(StatusCode::NO_CONTENT)
}
