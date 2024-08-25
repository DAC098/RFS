use std::fmt::Write as _;

use rfs_lib::ids;
use rfs_lib::sec::chacha;
use axum::http::StatusCode;
use axum::extract::{Query, Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;
use tokio_postgres::types::ToSql;
use base64::{Engine, engine::general_purpose::STANDARD};

use crate::error::{ApiError, ApiResult};
use crate::error::api::{ApiErrorKind, Context};
use crate::state::ArcShared;
use crate::sec::secrets::{Key, PeppersManager};
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::time;
use crate::sql;

pub async fn password_retrieve(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::SecSecrets,
        permission::Ability::Read
    ).await?;

    let mut known_versions = Vec::new();

    {
        let reader = state.sec().peppers().reader()?;

        known_versions.reserve(reader.len());

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

pub async fn password_create(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::SecSecrets,
        permission::Ability::Write
    ).await?;

    let data = Key::rand_key_data()?;
    let created = time::utc_now().context("failed to create timestamp")?;

    let key = Key::new(data, created);

    state.sec().peppers().update(key)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct PaswordVersionPath {
    version: u64
}

pub async fn password_retrieve_version(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PaswordVersionPath { version }): Path<PaswordVersionPath>
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::SecSecrets,
        permission::Ability::Read
    ).await?;

    let peppers = state.sec().peppers();

    let (data, created) = peppers.get_cb(&version, |result| {
        let Some(found) = result? else {
            return Err(ApiError::api(ApiErrorKind::SecretNotFound));
        };

        Ok(found.clone().into_tuple())
    })?;

    let conn = state.pool().get().await?;

    let count = conn.execute(
        "select auth_password.user_id from auth_password where auth_password.version = $1",
        &[&(version as i64)]
    ).await?;

    let created = time::utc_to_chrono_datetime(&created)
        .context("timestamp error for password key")?;

    Ok(rfs_api::Payload::new(rfs_api::sec::secrets::PasswordVersion {
        version,
        created,
        data: data.into(),
        in_use: count
    }))
}

fn password_get_next_avail(
    version: &u64,
    manager: &PeppersManager
) -> ApiResult<(Key, Option<(u64, Key)>)> {
    let reader = manager.reader()?;

    let found = reader.get(&version)
        .kind(ApiErrorKind::SecretNotFound)?;

    let mut iter = reader.iter();

    while let Some((ver, key)) = iter.next_back() {
        if ver != version {
            return Ok((found.clone(), Some((*ver, key.clone()))));
        }
    }

    Ok((found.clone(), None))
}

fn write_sql_array<'a, V>(
    sql: &mut String,
    value: &'a V,
    params: &mut sql::ParamsVec<'a>,
    with_comma: bool
)
where
    V: ToSql + Sync
{
    if !with_comma {
        write!(sql, "${}", sql::push_param(params, value)).unwrap();
    } else {
        write!(sql, ",${}", sql::push_param(params, value)).unwrap();
    }
}

pub async fn password_rotate_deletion(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PaswordVersionPath { version }): Path<PaswordVersionPath>
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::SecSecrets,
        permission::Ability::Write,
    ).await?;

    let (to_drop, maybe) = password_get_next_avail(&version, state.sec().peppers())?;

    let transaction = conn.transaction().await?;

    let records = transaction.query_raw(
        "\
        select auth_password.user_id, \
               auth_password.hash \
        from auth_password \
        where auth_password.version = $1",
        &[&(version as i64)]
    ).await?;

    futures::pin_mut!(records);

    let max_size = 10usize;

    loop {
        let mut records_empty = false;
        let mut batch = Vec::with_capacity(max_size);
        let mut batch_user_id = String::new();
        let mut batch_version = String::new();
        let mut batch_hash = String::new();
        let mut batch_params: sql::ParamsVec = Vec::with_capacity(max_size * 3);

        for _ in 0..max_size {
            let Some(row) = records.try_next().await? else {
                records_empty = true;
                break;
            };

            let user_id: ids::UserId = row.get(0);
            let hash: String = row.get(1);

            let decoded = STANDARD.decode(hash).unwrap();
            let decrypted = chacha::decrypt_data(to_drop.data(), decoded)?;

            let (ver, encrypted) = if let Some((ver, key)) = &maybe {
                (*ver, chacha::encrypt_data(key.data(), decrypted)?)
            } else {
                (0, decrypted)
            };

            let encoded = STANDARD.encode(encrypted);

            batch.push((user_id, ver as i64, encoded));
        }

        let mut iter = batch.iter();

        if let Some((id, v, h)) = iter.next() {
            tracing::debug!("id: {id} | v: {v} | h: {h}");

            write_sql_array(&mut batch_user_id, id, &mut batch_params, false);
            write_sql_array(&mut batch_version, v, &mut batch_params, false);
            write_sql_array(&mut batch_hash, h, &mut batch_params, false);

            for (id, v, h) in iter {
                tracing::debug!("id: {id} | v: {v} | h: {h}");

                write_sql_array(&mut batch_user_id, id, &mut batch_params, true);
                write_sql_array(&mut batch_version, v, &mut batch_params, true);
                write_sql_array(&mut batch_hash, h, &mut batch_params, true);
            }

            tracing::debug!("batch_user_id: \"{batch_user_id}\"");
            tracing::debug!("batch_version: \"{batch_version}\"");
            tracing::debug!("batch_hash: \"{batch_hash}\"");

            let query = format!("\
                with to_update as (\
                    select * \
                    from unnest(\
                        ARRAY[{batch_user_id}]::bigint[], \
                        ARRAY[{batch_version}]::bigint[], \
                        ARRAY[{batch_hash}]\
                    ) as t(user_id, version, hash)\
                ) \
                update auth_password set \
                    user_id = to_update.user_id, \
                    version = to_update.version, \
                    hash = to_update.hash \
                from to_update \
                where to_update.user_id = auth_password.user_id");

            transaction.execute(&query, &batch_params).await?;
        }

        if records_empty {
            break;
        }
    }

    transaction.commit().await?;

    state.sec()
        .peppers()
        .delete(&version)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn session_retrieve(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::SecSecrets,
        permission::Ability::Read,
    ).await?;

    let session_keys = state.sec()
        .session_info()
        .keys()
        .inner();
    let mut known_keys;

    {
        let Ok(reader) = session_keys.read() else {
            return Err(ApiError::new().source("session keys rwlock poisoned"));
        };

        known_keys = Vec::with_capacity(reader.stored());

        for key in reader.iter() {
            let created = time::utc_to_chrono_datetime(key.created())
                .context("timestamp error for session key")?;

            known_keys.push(rfs_api::sec::secrets::SessionListItem { created });
        }
    }

    Ok(rfs_api::Payload::new(known_keys))
}

pub async fn session_create(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::SecSecrets,
        permission::Ability::Write,
    ).await?;

    let wrapper = state.sec().session_info().keys();
    let data = Key::rand_key_data()?;
    let created = time::utc_now().context("timestamp error for session key")?;

    let key = Key::new(data, created);

    {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(ApiError::new().source("session keys rwlock poisoned"));
        };

        writer.push(key);
    }

    wrapper.save().context("failed to save session secret")?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct SessionDeleteQuery {
    amount: Option<usize>
}

pub async fn session_delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Query(query): Query<SessionDeleteQuery>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::SecSecrets,
        permission::Ability::Write
    ).await?;

    let wrapper = state.sec().session_info().keys();

    let Some(mut amount) = query.amount else {
        return Ok(StatusCode::NO_CONTENT);
    };

    if amount == 0 {
        return Ok(StatusCode::NO_CONTENT);
    }

    {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(ApiError::new().source("session keys rwlock poisoned"));
        };

        while amount > 0 {
            if let None = writer.pop() {
                break;
            }

            amount -= 1;
        }
    }

    wrapper.save().context("failed to save session secret")?;

    Ok(StatusCode::NO_CONTENT)
}
