use std::collections::HashSet;
use std::fmt::Write;

use rfs_lib::ids;

use axum::http::StatusCode;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use deadpool_postgres::GenericClient;
use futures::{StreamExt, TryStream, TryStreamExt};
use serde::Deserialize;
use tokio_postgres::{Error as PgError};

use crate::error::{ApiError, ApiResult};
use crate::error::api::{Detail, ApiErrorKind, Context};
use crate::sec::authn::initiator;
use crate::sec::authz::permission::{Rbac, Role, Permission, Ability, Scope};
use crate::sql;
use crate::db;
use crate::routing::query::PaginationQuery;

pub async fn retrieve(
    db::Conn(conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::RoleId>>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Read,
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsArray<2> = [&last_id, &limit];

        conn.query_raw(
            "\
            select id, \
                   name \
            from authz_roles \
            where authz_roles.id > $1 \
            order by authz_roles.id \
            limit $2",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsArray<2> = [&limit, &offset_num];

        conn.query_raw(
            "\
            select authz_roles.id, \
                   authz_roles.name \
            from authz_roles \
            order by authz_roles.id \
            limit $1 \
            offset $2",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::sec::roles::RoleListItem {
            id: row.get(0),
            name: row.get(1),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn create(
    db::Conn(mut conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<rfs_api::sec::roles::CreateRole>
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let result = match transaction.query_one(
        "\
        insert into authz_roles (name) \
        values ($1) \
        returning id",
        &[&json.name]
    ).await {
        Ok(r) => r,
        Err(err) => {
            if let Some(constraint) = sql::unique_constraint_error(&err) {
                if constraint == "authz_roles_name_key" {
                    return Err(ApiError::from((
                        ApiErrorKind::AlreadyExists,
                        Detail::with_key("name")
                    )));
                }
            }

            return Err(err.into());
        }
    };

    let role_id: ids::RoleId = result.get(0);

    if json.permissions.len() == 0 {
        transaction.commit().await?;

        return Ok((
            StatusCode::CREATED,
            rfs_api::Payload::new(rfs_api::sec::roles::Role {
                id: role_id,
                name: json.name,
                permissions: Vec::new()
            })
        ));
    }

    let mut first = true;
    let mut provided: HashSet<(Ability, Scope)> = HashSet::new();
    let mut query = String::from("insert into authz_permissions (role_id, scope, ability) values");
    let mut params: sql::ParamsVec = vec![&role_id];

    for given in &json.permissions {
        let pair = (given.ability.clone(), given.scope.clone());

        if provided.contains(&pair) {
            continue;
        }

        provided.insert(pair);

        if first {
            write!(
                &mut query,
                " ($1, ${}, ${})",
                sql::push_param(&mut params, &given.scope),
                sql::push_param(&mut params, &given.ability)
            )?;

            first = false;
        } else {
            write!(
                &mut query,
                ", ($1, ${}, ${})",
                sql::push_param(&mut params, &given.scope),
                sql::push_param(&mut params, &given.ability)
            )?;
        }
    }

    transaction.execute(query.as_str(), params.as_slice()).await?;
    transaction.commit().await?;

    let permissions = provided.into_iter()
        .map(|(ability,scope)| rfs_api::sec::roles::Permission {
            scope,
            ability
        })
        .collect();

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(rfs_api::sec::roles::Role {
            id: role_id,
            name: json.name,
            permissions
        })
    ))
}

#[derive(Deserialize)]
pub struct PathParams {
    role_id: i64
}

pub async fn retrieve_id(
    db::Conn(conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
) -> ApiResult<rfs_api::Payload<rfs_api::sec::roles::Role>> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Read,
    ).await?;

    let (role, permissions) = match tokio::try_join!(
        Role::retrieve(&conn, &role_id),
        Permission::stream_by_role_id(&conn, &role_id)
    ) {
        Ok((Some(role), permissions)) => (role, permissions),
        Ok((None, _)) => return Err(ApiError::from(ApiErrorKind::RoleNotFound)),
        Err(err) => return Err(err.into()),
    };

    let mut rtn: rfs_api::sec::roles::Role = role.into();

    futures::pin_mut!(permissions);

    while let Some(perm) = permissions.try_next().await? {
        rtn.permissions.push(perm.into());
    }

    Ok(rfs_api::Payload::new(rtn))
}

fn map_permission_tuple((scope, ability): (Scope, Ability)) -> rfs_api::sec::roles::Permission {
    rfs_api::sec::roles::Permission {
        scope,
        ability
    }
}

pub async fn update_id(
    db::Conn(mut conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::UpdateRole>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let original = Role::retrieve(&transaction, &role_id)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.name.is_none() && json.permissions.is_none() {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let name = if let Some(name) = json.name {
        match transaction.execute(
            "\
            update authz_roles \
            set name = $2 \
            where id = $1",
            &[&role_id, &name]
        ).await {
            Ok(_) => {},
            Err(err) => {
                if let Some(constraint) = sql::unique_constraint_error(&err) {
                    if constraint == "authz_roles_name_key" {
                        return Err(ApiError::from((
                            ApiErrorKind::AlreadyExists,
                            Detail::with_key("name")
                        )));
                    }
                }

                return Err(err.into());
            }
        }

        name
    } else {
        original.name
    };

    let mut changed_permissions = None;

    if let Some(permissions) = &json.permissions {
        // not sure how to go about this since authz_permissions has a composite
        // key so we cannot easily reference a specific row to drop.
        transaction.execute(
            "delete from authz_permissions where role_id = $1",
            &[&role_id]
        ).await?;

        if permissions.len() == 0 {
            changed_permissions = Some(HashSet::new());
        } else {
            let mut first = true;
            let mut provided: HashSet<(Scope, Ability)> = HashSet::new();
            let mut query = String::from("insert into authz_permissions (role_id, scope, ability) values");
            let mut params: sql::ParamsVec = vec![&role_id];

            for given in permissions {
                let pair = (given.scope.clone(), given.ability.clone());

                if provided.contains(&pair) {
                    continue;
                }

                provided.insert(pair);

                if first {
                    write!(
                        &mut query,
                        " ($1, ${}, ${})",
                        sql::push_param(&mut params, &given.scope),
                        sql::push_param(&mut params, &given.ability)
                    )?;

                    first = false;
                } else {
                    write!(
                        &mut query,
                        ", ($1, ${}, ${})",
                        sql::push_param(&mut params, &given.scope),
                        sql::push_param(&mut params, &given.ability)
                    )?;
                }
            }

            transaction.execute(query.as_str(), params.as_slice()).await?;

            changed_permissions = Some(provided);
        }
    }

    transaction.commit().await?;

    clear_via_role(&rbac, &conn, &role_id).await?;

    let permissions = if let Some(changed) = changed_permissions {
        changed.into_iter()
            .map(map_permission_tuple)
            .collect()
    } else {
        let result = Permission::stream_by_role_id(&conn, &role_id)
            .await
            .context("failed to retrieve permissions for role")?;

        futures::pin_mut!(result);

        let mut list = Vec::new();

        while let Some(perm) = result.try_next().await? {
            list.push(perm.into());
        }

        list
    };

    Ok(rfs_api::Payload::new(rfs_api::sec::roles::Role {
        id: role_id,
        name,
        permissions,
    }))
}

pub async fn delete_id(
    db::Conn(mut conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let _original = Role::retrieve(&transaction, &role_id)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    let query_params: sql::ParamsArray<1> = [&role_id];

    let result = tokio::try_join!(
        transaction.execute(
            "delete from authz_permissions where role_id = $1",
            &query_params
        ),
        transaction.execute(
            "delete from group_roles where role_id = $1",
            &query_params
        ),
        transaction.execute(
            "delete from user_roles where role_id = $1",
            &query_params
        )
    );

    if let Err(err) = result {
        return Err(err.into());
    }

    let _ = transaction.execute(
        "delete from authz_roles where id = $1",
        &query_params
    ).await?;

    transaction.commit().await?;

    clear_via_role(&rbac, &conn, &role_id).await?;

    Ok(StatusCode::OK)
}

pub async fn retreive_id_users(
    db::Conn(conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::UserId>>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Read
    ).await?;

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
        Ok((None, _)) => return Err(ApiError::from(ApiErrorKind::RoleNotFound)),
        Err(err) => return Err(ApiError::from(err)),
    };

    futures::pin_mut!(result);

    let mut users = Vec::new();

    while let Some(row) = result.try_next().await? {
        users.push(rfs_api::sec::roles::RoleUser {
            id: row.get(0)
        });
    }

    Ok(rfs_api::Payload::from((pagination, users)))
}

pub async fn add_id_users(
    db::Conn(mut conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::AddRoleUser>
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    Role::retrieve(&transaction, &role_id)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.ids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
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

    for user_id in json.ids {
        rbac.clear_id(&user_id);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_id_users(
    db::Conn(mut conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::DropRoleUser>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    Role::retrieve(&transaction, &role_id)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.ids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let _ = transaction.execute(
        "\
        delete from user_roles \
        where role_id = $1 and \
              user_id = any($2)",
        &[&role_id, &json.ids]
    ).await?;

    transaction.commit().await?;

    for user_id in json.ids {
        rbac.clear_id(&user_id);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn retrieve_id_groups(
    db::Conn(conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::GroupId>>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Read,
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);
    let offset_num = limit.sql_offset(offset);
    let params: sql::ParamsArray<3>;
    let query: &str;

    let maybe_last_id;

    if let Some(last_id) = last_id {
        maybe_last_id = last_id;

        params = [&role_id, &maybe_last_id, &limit];
        query = "\
            select group_roles.group_id \
            from group_roles \
            where group_roles.role_id = $1 and \
                  group_roles.group_id > $2 \
            order by group_roles.group_id \
            limit $3";
    } else {
        pagination.set_offset(offset);

        params = [&role_id, &limit, &offset_num];
        query = "\
            select group_roles.group_id \
            from group_roles \
            where group_roles.role_id = $1
            order by group_roles.group_id \
            limit $2 \
            offset $3";
    }

    let role_fut = Role::retrieve(&conn, &role_id);
    let groups_fut = conn.query_raw(query, params);

    let result = match tokio::try_join!(role_fut, groups_fut) {
        Ok((Some(_), rows)) => rows,
        Ok((None, _)) => return Err(ApiError::from(ApiErrorKind::RoleNotFound)),
        Err(err) => return Err(ApiError::from(err)),
    };

    futures::pin_mut!(result);

    let mut groups = Vec::new();

    while let Some(row) = result.try_next().await? {
        groups.push(rfs_api::sec::roles::RoleGroup {
            id: row.get(0)
        });
    }

    Ok(rfs_api::Payload::from((pagination, groups)))
}

pub async fn add_id_groups(
    db::Conn(mut conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::AddRoleGroup>
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write,
    ).await?;

    let transaction = conn.transaction().await?;

    Role::retrieve(&transaction, &role_id)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.ids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let query = "\
        insert into group_roles (role_id, group_id) \
        select $1 as role_id, \
               groups.id as group_id \
        from groups \
        where groups.id = any($2) \
        on conflict on constraint group_roles_pkey do nothing";
    let params: sql::ParamsArray<2> = [&role_id, &json.ids];

    transaction.execute(query, &params).await?;

    let users = users_multi_group(&transaction, &json.ids)
        .await
        .context("failed to retrieve users attached to groups")?;

    futures::pin_mut!(users);

    while let Some(user) = users.try_next().await? {
        rbac.clear_id(&user);
    }

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_id_groups(
    db::Conn(mut conn): db::Conn,
    rbac: Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::DropRoleGroup>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    Role::retrieve(&transaction, &role_id)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.ids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    transaction.execute(
        "\
        delete from group_roles \
        where role_id = $1 and \
              group_id = any($2)",
        &[&role_id, &json.ids]
    ).await?;

    transaction.commit().await?;

    let users = users_multi_group(&conn, &json.ids)
        .await
        .context("failed to retrieve users attached to groups")?;

    futures::pin_mut!(users);

    while let Some(user) = users.try_next().await? {
        rbac.clear_id(&user);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn users_multi_group(
    conn: &impl GenericClient,
    group_ids: &[ids::GroupId]
) -> Result<impl TryStream<Item = Result<ids::UserId, PgError>>, PgError> {
    let result = conn.query_raw(
        "\
        select distinct on (user_id) user_id \
        from group_users \
        where group_id = any($1)",
        &[group_ids]
    ).await?;

    Ok(result.map(|row_result| row_result.map(
        |row| row.get::<usize, ids::UserId>(0)
    )))
}

async fn users_via_groups(
    conn: &impl GenericClient,
    role_id: &ids::RoleId,
) -> ApiResult<impl TryStream<Item = Result<ids::UserId, PgError>>> {
    let result = conn.query_raw(
        "\
        select group_users.user_id \
        from group_users \
        right join group_roles on \
            group_users.group_id = group_roles.group_id \
        left join authz_roles on \
            group_roles.role_id = authz_roles.id \
        where authz_roles.id = $1",
        &[role_id]
    ).await.context("failed to retrieve group users attahced to role")?;

    Ok(result.map(|row_result| row_result.map(
        |row| row.get::<usize, ids::UserId>(0)
    )))
}

async fn users(
    conn: &impl GenericClient,
    role_id: &ids::RoleId
) -> ApiResult<impl TryStream<Item = Result<ids::UserId, PgError>>> {
    let result = conn.query_raw(
        "\
        select user_roles.user_id \
        from user_roles \
        left join authz_roles on \
            user_roles.role_id = authz_roles.id \
        where authz_roles.id = $1",
        &[role_id]
    ).await.context("failed to retrieve users attached to role")?;

    Ok(result.map(|row_result| row_result.map(
        |row| row.get::<usize, ids::UserId>(0)
    )))
}

async fn clear_via_role(
    rbac: &Rbac,
    conn: &impl GenericClient,
    role_id: &ids::RoleId
) -> ApiResult<()> {
    let (users, users_via_groups) = tokio::try_join!(
        users(conn, role_id),
        users_via_groups(conn, role_id),
    )?;

    futures::pin_mut!(users);
    futures::pin_mut!(users_via_groups);

    while let Some(user_id) = users.try_next().await? {
        rbac.clear_id(&user_id);
    }

    while let Some(user_id) = users_via_groups.try_next().await? {
        rbac.clear_id(&user_id);
    }

    Ok(())
}
