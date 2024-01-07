use std::collections::HashMap;

use rfs_lib::ids;
use rfs_api::fs::storage::{StorageListItem, CreateStorage, CreateStorageType};
use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::storage;
use crate::fs;
use crate::tags;

pub mod storage_id;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    _headers: HeaderMap,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let params = [initiator.user().id()];

    let fut = conn.query_raw(
        "\
        select storage.id, \
               storage.name, \
               storage.user_id, \
               storage.s_data, \
        from storage \
        where storage.user_id = $1 \
        order by storage.id",
        &params
    );
    let tags_fut = conn.query_raw(
        "\
        select storage_tags.storage_id, \
               storage_tags.tag, \
               storage_tags.value \
        from storage_tags \
            join storage on \
                storage_tags.storage_id = storage.id \
        where storage.user_i = $1 \
        order storage_tags.storage_id",
        &params
    );

    let (result, tags_result) = tokio::try_join!(fut, tags_fut)?;

    futures::pin_mut!(result);
    futures::pin_mut!(tags_result);

    let mut tags_finished = false;
    let mut list = Vec::with_capacity(10);
    let mut current: Option<(ids::StorageId, String, Option<String>)> = None;

    while let Some(row) = result.try_next().await? {
        let mut item = StorageListItem {
            id: row.get(0),
            name: row.get(1),
            user_id: row.get(2),
            type_: sql::de_from_sql(row.get(3)),
            tags: HashMap::new()
        };

        if let Some((ref_id, _tag, _value)) = &current {
            if item.id == *ref_id {
                let taken = current.take().unwrap();
                item.tags.insert(taken.1, taken.2);
            } else {
                list.push(item);

                continue;
            }
        } else if tags_finished {
            list.push(item);

            continue;
        }

        loop {
            let Some(tag_row) = tags_result.try_next().await? else {
                tags_finished = true;
                break;
            };

            let row_id: ids::StorageId = tag_row.get(0);

            if item.id == row_id {
                item.tags.insert(row.get(1), row.get(2));
            } else {
                current = Some((row_id, row.get(1), row.get(2)));
                break;
            }
        }

        list.push(item);
    }

    Ok(rfs_api::ListPayload::with_vec(list))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<CreateStorage>,
) -> error::Result<impl IntoResponse> {
    tracing::event!(
        tracing::Level::DEBUG,
        "creating new storage medium"
    );

    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let type_: storage::types::Type = match json.type_ {
        CreateStorageType::Local { path } => {
            if !path.is_absolute() {
                return Err(error::Error::api(error::StorageKind::NotAbsolutePath));
            }

            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::NotFound => {
                            return Err(error::Error::api(error::StorageKind::DirNotFound));
                        },
                        _ => {
                            return Err(err.into())
                        }
                    }
                }
            };

            if !metadata.is_dir() {
                return Err(error::Error::api(error::StorageKind::NotDirectory));
            }

            tokio::fs::create_dir_all(&path).await?;

            storage::types::Type::Local(storage::types::Local { path })
        }
    };

    let transaction = conn.transaction().await?;

    let id = state.ids().wait_storage_id()?;
    let created = chrono::Utc::now();

    if !rfs_lib::storage::name_valid(&json.name) {
        return Err(error::Error::api((
            error::GeneralKind::ValidationFailed,
            error::Detail::with_key("name")
        )));
    };

    if storage::name_check(&transaction, &initiator.user().id(), &json.name).await?.is_some() {
        return Err(error::Error::api((
            error::GeneralKind::AlreadyExists,
            error::Detail::with_key("name")
        )));
    }

    {
        let pg_type = sql::ser_to_sql(&type_);

        transaction.execute(
            "\
            insert into storage (id, user_id, name, s_data, created) values \
            ($1, $2, $3, $4, $5)",
            &[&id, initiator.user().id(), &json.name, &pg_type, &created]
        ).await?;

        if !tags::validate_map(&json.tags) {
            return Err(error::Error::api(error::TagKind::InvalidTags));
        }

        tags::create_tags(&transaction, "storage_tags", "storage_id", &id, &json.tags).await?;
    }

    let medium = storage::Medium {
        id,
        name: json.name,
        user_id: initiator.user().id().clone(),
        type_,
        tags: json.tags,
        created,
        updated: None,
        deleted: None,
    };

    let created = chrono::Utc::now();
    let id = state.ids().wait_fs_id()?;
    let storage = storage::fs::Storage::Local(storage::fs::Local {
        id: medium.id.clone()
    });

    {
        let pg_s_data = sql::ser_to_sql(&storage);

        transaction.execute(
            "\
            insert into fs (id, user_id, fs_type, s_data, created) values \
            ($1, $2, $3, $4, $5)",
            &[&id, initiator.user().id(), &fs::consts::ROOT_TYPE, &pg_s_data, &created]
        ).await?;
    }

    transaction.commit().await?;

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(medium.into_schema())
    ))
}
