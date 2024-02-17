use rfs_lib::ids;
use rfs_lib::query::{Limit, Offset};

use axum::http::StatusCode;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission::{self, Role};
use crate::sql;

#[derive(Deserialize)]
pub struct PathParams {
    role_id: ids::RoleId,
}

#[derive(Deserialize)]
pub struct GetQuery {
    #[serde(default)]
    limit: Limit,

    #[serde(default)]
    offset: Offset,

    last_id: Option<ids::UserId>
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    Query(GetQuery { limit, offset, last_id }): Query<GetQuery>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecRoles,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let mut pagination = rfs_api::Pagination::from(&limit);
    let offset_num = limit.sql_offset(offset);
    let params: sql::ParamsArray<3>;
    let query: &str;

    let maybe_last_id;

    if let Some(last_id) = last_id {
        maybe_last_id = last_id;

        params = [&role_id, &maybe_last_id, &limit];
        query = "\
            select user_roles.user_id \
            from user_roles \
            where user_roles.role_id = $1 and \
                  user_roles.user_id > $2 \
            order by user_roles.user_id \
            limit $3";
    } else {
        pagination.set_offset(offset);

        params = [&role_id, &limit, &offset_num];
        query = "\
            select user_roles.user_id \
            from user_roles \
            where user_roles.role_id = $1 \
            order by user_roles.user_id \
            limit $2 \
            offset $3";
    }

    let role_fut = Role::retrieve(&conn, &role_id);
    let users_fut = conn.query_raw(query, params);

    let result = match tokio::try_join!(role_fut, users_fut) {
        Ok((Some(_), rows)) => rows,
        Ok((None, _)) => {
            return Err(error::Error::api(error::ApiErrorKind::RoleNotFound));
        }
        Err(err) => {
            return Err(error::Error::from(err));
        }
    };

    futures::pin_mut!(result);

    let mut users = Vec::new();

    while let Some(row) = result.try_next().await? {
        users.push(rfs_api::sec::roles::RoleUser {
            id: row.get(0)
        });
    }

    Ok(rfs_api::Payload::new(users))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::AddRoleUser>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecRoles,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let transaction = conn.transaction().await?;

    let Some(_role) = Role::retrieve(&transaction, &role_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::RoleNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::ApiErrorKind::NoWork));
    }

    let query = "\
        insert into user_roles (role_id, user_id) \
        select $1 as role_id, \
               users.id as user_id \
        from users \
        where users.id = any($2) \
        on conflict on constraint user_roles_pkey do nothing";
    let params: sql::ParamsArray<2> = [&role_id, &json.ids];

    let _ = transaction.execute(query, &params).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::DropRoleUser>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::SecRoles,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let transaction = conn.transaction().await?;

    let Some(_role) = Role::retrieve(&transaction, &role_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::RoleNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::ApiErrorKind::NoWork));
    }

    let _ = transaction.execute(
        "\
        delete from user_roles \
        where role_id = $1 and \
              user_id = any($2)",
        &[&role_id, &json.ids]
    ).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
