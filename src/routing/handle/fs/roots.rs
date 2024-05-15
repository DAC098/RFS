use rfs_lib::ids;
use rfs_lib::query::{Limit, Offset};
use rfs_api::fs::{RootMin, ItemMin};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::fs;

#[derive(Deserialize)]
pub struct GetQuery {
    #[serde(default)]
    limit: Limit,

    #[serde(default)]
    offset: Offset,

    last_id: Option<ids::FSId>,
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Query(GetQuery { limit, offset, last_id }): Query<GetQuery>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![&initiator.user.id, &last_id, &fs::consts::ROOT_TYPE, &limit];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.user_id = $1 and \
                  fs.id > $2 and \
                  fs.fs_type = $3 \
            order by fs.id \
            limit $4",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![&initiator.user.id, &fs::consts::ROOT_TYPE, &limit, &offset_num];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.user_id = $1 and \
                  fs.fs_type = $2 \
            order by fs.id \
            limit $3 \
            offset $4",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(limit as usize);

    while let Some(row) = result.try_next().await? {
        let item = ItemMin::Root(RootMin {
            id: row.get(0),
            user_id: row.get(1),
            created: row.get(2),
            updated: row.get(3),
        });

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}
