use rfs_lib::ids::GroupId;
use rfs_lib::query::{Limit, Offset};

use axum::http::StatusCode;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use tokio_postgres::error::SqlState;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;

pub mod group_id;

#[derive(Deserialize)]
pub struct GetQuery {
    #[serde(default)]
    limit: Limit,

    #[serde(default)]
    offset: Offset,

    last_id: Option<GroupId>
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
        permission::Scope::UserGroup,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![&last_id, &limit];

        conn.query_raw(
            "\
            select groups.id, \
                   groups.name \
            from groups \
            where groups.id > $1 \
            order by groups.id \
            limit $2",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![&limit, &offset_num];

        conn.query_raw(
            "\
            select groups.id, \
                   groups.name \
            from groups \
            order by groups.id \
            limit $1 \
            offset $2",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::users::groups::ListItem {
            id: row.get(0),
            name: row.get(1),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<rfs_api::users::groups::CreateGroup>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::UserGroup,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let name = json.name;
    let created = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    let result = match transaction.query_one( "\
        insert into groups (name, created) \
        values ($1, $2) \
        returning id",
        &[&name, &created]
    ).await {
        Ok(r) => r,
        Err(err) => {
            let Some(db_error) = err.as_db_error() else {
                return Err(err.into());
            };

            if *db_error.code() == SqlState::UNIQUE_VIOLATION {
                let Some(constraint) = db_error.constraint() else {
                    return Err(err.into());
                };

                if constraint == "groups_name_key" {
                    return Err(error::Error::api((
                        error::ApiErrorKind::AlreadyExists,
                        error::Detail::with_key("name")
                    )));
                }
            }

            return Err(err.into());
        }
    };

    let rtn = rfs_api::users::groups::Group {
        id: result.get(0),
        name,
        created,
        updated: None
    };

    transaction.commit().await?;

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(rtn)
    ))
}
