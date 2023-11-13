use rfs_lib::{schema, actions};
use axum::http::StatusCode;
use axum::extract::{Query, Path, State};
use axum::response::IntoResponse;
use chrono::{Utc, DateTime};
use serde::Deserialize;

use crate::net::{self, error};
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
        initiator.user().id(),
        permission::Scope::SecSecrets,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::new()
            .status(StatusCode::UNAUTHORIZED)
            .kind("PermissionDeniend"));
    }

    let session_keys = state.sec().session_info().keys().inner();
    let mut known_keys;

    {
        let reader = session_keys.read()
            .map_err(|_| error::Error::new().source("session keys rwlock poisoned"))?;
        known_keys = Vec::with_capacity(reader.stored());

        for key in reader.iter() {
            known_keys.push(schema::sec::SessionListItem {
                created: time::utc_to_chrono_datetime(key.created())
                    .ok_or(error::Error::new()
                        .kind("TimestampError")
                        .message("failed to create timestamp for session key"))?
            });
        }
    }

    Ok(net::Json::new(rfs_lib::json::ListWrapper::with_vec(known_keys)))
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
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::new()
            .status(StatusCode::UNAUTHORIZED)
            .kind("PermissionDenied"));
    }

    let wrapper = state.sec().session_info().keys();
    let data = Key::rand_key_data()?;
    let Some(created) = time::utc_now() else {
        return Err(error::Error::new()
            .kind("TimestampError")
            .source("failed to create timestamp"));
    };

    let key = Key::new(data, created);

    {
        let mut writer = wrapper.inner().write()
            .map_err(|_| error::Error::new()
                .source("session keys rwlock poisoned"))?;

        writer.push(key);
    }

    wrapper.save()
        .map_err(|e| error::Error::new()
            .kind("FailedSavingSessionKeys")
            .message("failed to save updated session keys to file")
            .source(e))?;

    Ok(net::Json::empty())
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
        initiator.user().id(),
        permission::Scope::SecSecrets,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::new()
            .status(StatusCode::UNAUTHORIZED)
            .kind("PermissionDenied"));
    }

    let wrapper = state.sec().session_info().keys();

    let Some(mut amount) = query.amount else {
        return Ok(net::Json::empty());
    };

    if amount == 0 {
        return Ok(net::Json::empty());
    }

    {
        let mut writer = wrapper.inner().write()
            .map_err(|_| error::Error::new()
                .source("session keys rwlock poisoned"))?;

        while amount > 0 {
            if let None = writer.pop() {
                break;
            }

            amount -= 1;
        }
    }

    wrapper.save()
        .map_err(|e| error::Error::new()
            .kind("FailedSavingSessionKeys")
            .message("failed to save updated session keys to file")
            .source(e))?;

    Ok(net::Json::empty())
}
