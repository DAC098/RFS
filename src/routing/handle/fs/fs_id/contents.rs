use rfs_lib::ids;
use rfs_lib::query::{Limit, Offset};
use rfs_api::fs::{FileMin, DirectoryMin, RootMin, ItemMin};
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
pub struct PathParams {
    fs_id: ids::FSId
}

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
    Path(PathParams { fs_id }): Path<PathParams>,
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

    let Some(item) = fs::Item::retrieve(&conn, &fs_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::FileNotFound));
    };

    if *item.user_id() != initiator.user.id {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(container) = item.as_container() else {
        return Err(error::Error::api(error::ApiErrorKind::NotDirectory));
    };

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![container.id(), &last_id, &limit];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.storage_id, \
                   fs.parent, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.parent = $1 and fs.id > $2 \
            order by fs.id \
            limit $3",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![container.id(), &limit, &offset_num];

        conn.query_raw(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.storage_id, \
                   fs.parent, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.created, \
                   fs.updated \
            from fs \
            where fs.parent = $1 \
            order by fs.id \
            limit $2 \
            offset $3",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(limit as usize);

    while let Some(row) = result.try_next().await? {
        let fs_type = row.get(5);

        let item = match fs_type {
            fs::consts::ROOT_TYPE => {
                ItemMin::Root(RootMin {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage_id: row.get(2),
                    basename: row.get(4),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            fs::consts::FILE_TYPE => {
                ItemMin::File(FileMin {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage_id: row.get(2),
                    parent: row.get(3),
                    basename: row.get(4),
                    path: row.get(6),
                    size: sql::u64_from_sql(row.get(7)),
                    mime: sql::mime_from_sql(row.get(8), row.get(9)),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            fs::consts::DIR_TYPE => {
                ItemMin::Directory(DirectoryMin {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage_id: row.get(2),
                    parent: row.get(3),
                    basename: row.get(4),
                    path: row.get(6),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            _ => {
                panic!("unexpected fs_type when retrieving fs item. type: {fs_type}");
            }
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}
