use std::collections::HashMap;
use std::fmt::Write;

use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;
use tokio_postgres::types::Json as PgJson;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use lib::ids;
use lib::schema::storage::{StorageItem, StorageType, StorageListItem};
use lib::actions::storage::{CreateStorage, CreateStorageType};

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::util::sql;
use crate::storage;
use crate::fs;
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

    let type_: storage::types::Type = match json.type_ {
        CreateStorageType::Local { path } => {
            storage::types::Type::Local(storage::types::Local::build(path).await?)
        }
    };

    let transaction = conn.transaction().await?;

    let mut builder = storage::Medium::builder(
        state.ids().wait_storage_id()?,
        initiator.user().id().clone(),
        json.name,
        type_
    );

    builder.set_tags(json.tags);

    let storage = builder.build(&transaction).await?;
    let root = fs::Root::builder(
        state.ids().wait_fs_id()?,
        initiator.user().id().clone(),
        &storage
    ).build(&transaction).await?;

    transaction.commit().await?;

    let rtn = lib::json::Wrapper::new(storage.into_schema())
        .with_message("created storage");

    Ok(net::Json::new(rtn))
}
