use std::collections::HashMap;
use std::fmt::Write;

use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;
use tokio_postgres::types::Json as PgJson;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use lib::ids;
use lib::models::storage::{StorageItem, StorageType, StorageListItem};
use lib::actions::storage::{CreateStorage, CreateStorageType};

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::util::{sql, PgParams};
use crate::storage;
use crate::tags;

pub mod storage_id;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    headers: HeaderMap,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;
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

    let (mut result, mut tags_result) = tokio::try_join!(fut, tags_fut)?;

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

        if let Some((ref_id, tag, value)) = &current {
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

    let wrapper = lib::json::ListWrapper::with_vec(list);

    Ok(net::Json::new(wrapper))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<CreateStorage>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let id = state.ids().wait_storage_id()?;
    let created = chrono::Utc::now();
    let rtn_type;

    let s_data = match json.type_ {
        CreateStorageType::Local { path } => {
            if path.try_exists()? {
                if !path.is_dir() {
                    return Err(error::Error::new()
                        .status(StatusCode::BAD_REQUEST)
                        .kind("PathNotDirectory")
                        .message("the requested path is not a directory on the system"));
                }
            } else {
                tokio::fs::create_dir_all(&path).await?;
            }

            rtn_type = StorageType::Local {
                path: path.clone()
            };

            storage::Type::Local(storage::types::Local { path })
        }
    };

    if storage::name_check(&conn, initiator.user().id(), &json.name).await?.is_some() {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("StorageNameExists")
            .message("the requested name already exists"));
    }

    let transaction = conn.transaction().await?;

    let storage_json = PgJson(&s_data);
    let _ = transaction.execute(
        "\
        insert into storage (id, user_id, name, s_type, s_data, created) values \
        ($1, $2, $3, $4, $5)",
        &[&id, &initiator.user().id(), &json.name, &storage_json, &created]
    ).await?;

    tags::create_tags(&transaction, "storage_tags", "storage_id", &id, &json.tags).await?;

    transaction.commit().await?;

    let rtn = lib::json::Wrapper::new(StorageItem {
        id,
        name: json.name,
        user_id: initiator.user().id().clone(),
        type_: rtn_type,
        tags: json.tags,
        created,
        updated: None,
        deleted: None
    })
        .with_message("created storage");

    Ok(net::Json::new(rtn))
}
