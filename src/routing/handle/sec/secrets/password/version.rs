use std::fmt::Write as _;

use rfs_lib::ids;
use rfs_lib::sec::chacha;
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;
use tokio_postgres::types::ToSql;
use base64::{Engine, engine::general_purpose::STANDARD};

use crate::net::error;
use crate::state::ArcShared;

use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sec::secrets::{Key, PeppersManager};
use crate::time;
use crate::sql;

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

    let peppers = state.sec().peppers();

    let (data, created) = peppers.get_cb(&version, |result| {
        let Some(found) = result? else {
            return Err(error::Error::api(error::ApiErrorKind::SecretNotFound));
        };

        Ok(found.clone().into_tuple())
    })?;

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

fn get_version_and_next_avail(version: &u64, manager: &PeppersManager) -> error::Result<(Key, Option<(u64, Key)>)> {
    let reader = manager.reader()?;

    let Some(found) = reader.get(&version) else {
        return Err(error::Error::api(error::ApiErrorKind::SecretNotFound));
    };

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

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { version }): Path<PathParams>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecSecrets,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let (to_drop, maybe) = get_version_and_next_avail(&version, state.sec().peppers())?;

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
            write_sql_array(&mut batch_user_id, id, &mut batch_params, false);
            write_sql_array(&mut batch_version, v, &mut batch_params, false);
            write_sql_array(&mut batch_hash, h, &mut batch_params, false);

            for (id, v, h) in iter {
                write_sql_array(&mut batch_user_id, id, &mut batch_params, true);
                write_sql_array(&mut batch_version, v, &mut batch_params, true);
                write_sql_array(&mut batch_hash, h, &mut batch_params, true);
            }

            let query = format!("\
                with to_update as (\
                    select * \
                    from unnest(\
                        ARRAY[{batch_user_id}], \
                        ARRAY[{batch_version}], \
                        ARRAY[{batch_hash}]\
                    ) as t(user_id, version, hash)\
                ) \
                update auth_password set \
                    user_id = to_update.user_id, \
                    version = to_udpate.version, \
                    hash = to_update.hash \
                where to_update.user_id = auth_password.user_id");

            transaction.execute(&query, &batch_params).await?;
        }

        if records_empty {
            break;
        }
    }

    state.sec()
        .peppers()
        .delete(&version)?;

    Ok(StatusCode::NO_CONTENT)
}
