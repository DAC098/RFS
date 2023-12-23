use std::fmt::Write;

use rfs_lib::{ids, schema, actions};
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission::{self, Role};
use crate::sql;

#[derive(Deserialize)]
pub struct PathParams {
    role_id: ids::RoleId,
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::SecRoles,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let Some(_role) = Role::retrieve(&conn, &role_id).await? else {
        return Err(error::Error::api(error::SecKind::RoleNotFound));
    };

    let query_params: sql::ParamsArray<1> = [&role_id];
    let result = conn.query_raw(
        "select group_id from group_roles where role_id = $1",
        [&role_id]
    ).await?;

    let mut users = Vec::new();

    futures::pin_mut!(result);

    while let Some(row) = result.try_next().await? {
        users.push(schema::sec::RoleGroup {
            id: row.get(0)
        });
    }

    let wrapper = rfs_lib::json::ListWrapper::with_vec(users);

    Ok(net::Json::new(wrapper))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<actions::sec::AddRoleGroup>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::SecRoles,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let transaction = conn.transaction().await?;

    let Some(_role) = Role::retrieve(&transaction, &role_id).await? else {
        return Err(error::Error::api(error::SecKind::RoleNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::GeneralKind::NoWork));
    }

    let query = "\
        insert into group_roles (role_id, group_id) \
        select $1 as role_id, \
               groups.id as group_id \
        from groups \
        where groups.id = any($2) \
        on conflict on constraint group_roles_pkey do nothing";
    let params: sql::ParamsArray<2> = [&role_id, &json.ids];

    let _ = transaction.execute(query, &params).await?;

    transaction.commit().await?;

    Ok(net::Json::empty())
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<actions::sec::DropRoleGroup>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::SecRoles,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let transaction = conn.transaction().await?;

    let Some(_role) = Role::retrieve(&transaction, &role_id).await? else {
        return Err(error::Error::api(error::SecKind::RoleNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::GeneralKind::NoWork));
    }

    let _ = transaction.execute(
        "\
        delete from group_roles \
        where role_id = $1 and \
              group_id = any($2)",
        &[&role_id, &json.ids]
    ).await?;

    transaction.commit().await?;

    Ok(net::Json::empty())
}
