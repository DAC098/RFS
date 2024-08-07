use std::collections::HashSet;
use std::fmt::Write;

use rfs_lib::ids;

use axum::http::StatusCode;
use axum::extract::{Path, State, Query};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{Detail, ApiErrorKind, Context};
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission::{self, Role, Permission, Ability, Scope};
use crate::sql;
use crate::routing::query::PaginationQuery;

pub async fn retrieve(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::RoleUid>>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    permission::api_ability(
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
            select authz_roles.uid, \
                   authz_roles.name \
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
            select authz_roles.uid, \
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
            uid: row.get(0),
            name: row.get(1),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn create(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<rfs_api::sec::roles::CreateRole>
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;
    let uid = ids::RoleUid::gen();

    let result = match transaction.query_one(
        "\
        insert into authz_roles (uid, name) \
        values ($1, $2) \
        returning id",
        &[&uid, &json.name]
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
                uid,
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
            uid,
            name: json.name,
            permissions
        })
    ))
}

#[derive(Deserialize)]
pub struct PathParams {
    role_uid: ids::RoleUid
}

pub async fn retrieve_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Read,
    ).await?;

    let role_params: sql::ParamsArray<1> = [&role_uid];

    let (role_result, permissions_result) = match tokio::try_join!(
        conn.query_opt(
            "select uid, name from authz_roles where uid = $1",
            &role_params
        ),
        conn.query_raw(
            "\
            select scope, \
                   ability \
            from authz_permissions \
                join authz_roles on authz_permissions.role_id = authz_roles.id \
            where authz_roles.uid = $1",
            [&role_uid]
        )
    ) {
        Ok((Some(role), permissions)) => (role, permissions),
        Ok((None, _)) => {
            return Err(ApiError::from(ApiErrorKind::RoleNotFound));
        },
        Err(err) => {
            return Err(err.into());
        }
    };

    let mut permissions = Vec::new();

    futures::pin_mut!(permissions_result);

    while let Some(row) = permissions_result.try_next().await? {
        permissions.push(rfs_api::sec::roles::Permission {
            scope: row.get(0),
            ability: row.get(1),
        });
    }

    let rtn = rfs_api::sec::roles::Role {
        uid: role_result.get(0),
        name: role_result.get(1),
        permissions,
    };

    Ok(rfs_api::Payload::new(rtn))
}

pub async fn update_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::UpdateRole>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let original = Role::retrieve_uid(&transaction, &role_uid)
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
            &[original.id.local(), &name]
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
            &[original.id.local()]
        ).await?;

        if permissions.len() == 0 {
            changed_permissions = Some(HashSet::new());
        } else {
            let mut first = true;
            let mut provided: HashSet<(Scope, Ability)> = HashSet::new();
            let mut query = String::from("insert into authz_permissions (role_id, scope, ability) values");
            let mut params: sql::ParamsVec = vec![original.id.local()];

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

    let permissions = if let Some(changed) = changed_permissions {
        changed.into_iter()
            .map(|(scope, ability)| rfs_api::sec::roles::Permission {
                scope,
                ability
            })
            .collect()
    } else {
        Permission::retrieve_by_role_id(&conn, original.id.local())
            .await?
            .into_iter()
            .map(|perm| rfs_api::sec::roles::Permission {
                scope: perm.scope,
                ability: perm.ability
            })
            .collect()
    };

    Ok(rfs_api::Payload::new(rfs_api::sec::roles::Role {
        uid: role_uid,
        name,
        permissions,
    }))
}

pub async fn delete_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let original = Role::retrieve_uid(&transaction, &role_uid)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    let query_params: sql::ParamsArray<1> = [original.id.local()];

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

    Ok(StatusCode::OK)
}

pub async fn retreive_id_users(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::UserUid>>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    permission::api_ability(
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

        params = [&role_uid, &maybe_last_id, &limit];
        query = "\
            select users.uid \
            from user_roles \
            left join users on \
                user_roles.user_id = users.id \
            left join authz_roles on \
                user_roles.role_id = authz_roles.id \
            where authz_roles.uid = $1 and \
                  user_roles.user_id > (\
                select users.id \
                from users \
                where users.uid = $2) \
            order by user_roles.user_id \
            limit $3";
    } else {
        pagination.set_offset(offset);

        params = [&role_uid, &limit, &offset_num];
        query = "\
            select users.uid \
            from user_roles \
            left join users on \
                user_roles.user_id = users.id \
            left join authz_roles on \
                user_roles.role_id = authz_roles.id \
            where authz_roles.uid = $1
            order by user_roles.user_id \
            limit $2 \
            offset $3";
    }

    let role_fut = Role::retrieve_uid(&conn, &role_uid);
    let users_fut = conn.query_raw(query, params);

    let result = match tokio::try_join!(role_fut, users_fut) {
        Ok((Some(_), rows)) => rows,
        Ok((None, _)) => {
            return Err(ApiError::from(ApiErrorKind::RoleNotFound));
        }
        Err(err) => {
            return Err(ApiError::from(err));
        }
    };

    futures::pin_mut!(result);

    let mut users = Vec::new();

    while let Some(row) = result.try_next().await? {
        users.push(rfs_api::sec::roles::RoleUser {
            uid: row.get(0)
        });
    }

    Ok(rfs_api::Payload::from((pagination, users)))
}

pub async fn add_id_users(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::AddRoleUser>
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let role = Role::retrieve_uid(&transaction, &role_uid)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.uids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let query = "\
        insert into user_roles (role_id, user_id) \
        select $1 as role_id, \
               users.id as user_id \
        from users \
        where users.uid = any($2) \
        on conflict on constraint user_roles_pkey do nothing";
    let params: sql::ParamsArray<2> = [role.id.local(), &json.uids];

    let _ = transaction.execute(query, &params).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_id_users(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::DropRoleUser>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let role = Role::retrieve_uid(&transaction, &role_uid)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.uids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let _ = transaction.execute(
        "\
        delete from user_roles \
        using users \
        where role_id = $1 and \
              user_id = users.id and \
              users.uid = any($2)",
        &[role.id.local(), &json.uids]
    ).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn retrieve_id_groups(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::GroupUid>>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    permission::api_ability(
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

        params = [&role_uid, &maybe_last_id, &limit];
        query = "\
            select groups.uid \
            from group_roles \
            left join groups on \
                group_roles.group_id = groups.id \
            left join authz_roles on \
                group_roles.role_id = authz_roles.id \
            where authz_roles.uid = $1 and \
                  group_roles.group_id > (\
                      select group.id \
                      from groups \
                      where groups.uid = $2) \
            order by group_roles.group_id \
            limit $3";
    } else {
        pagination.set_offset(offset);

        params = [&role_uid, &limit, &offset_num];
        query = "\
            select groups.uid \
            from group_roles \
            left join groups on \
                group_roles.group_id = groups.id \
            left join authz_roles on \
                group_roles.role_id = authz_roles.id \
            where authz_roles.uid = $1 \
            order by group_roles.group_id \
            limit $2 \
            offset $3";
    }

    let role_fut = Role::retrieve_uid(&conn, &role_uid);
    let groups_fut = conn.query_raw(query, params);

    let result = match tokio::try_join!(role_fut, groups_fut) {
        Ok((Some(_), rows)) => rows,
        Ok((None, _)) => {
            return Err(ApiError::from(ApiErrorKind::RoleNotFound));
        }
        Err(err) => {
            return Err(ApiError::from(err));
        }
    };

    futures::pin_mut!(result);

    let mut groups = Vec::new();

    while let Some(row) = result.try_next().await? {
        groups.push(rfs_api::sec::roles::RoleGroup {
            uid: row.get(0)
        });
    }

    Ok(rfs_api::Payload::from((pagination, groups)))
}

pub async fn add_id_groups(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::AddRoleGroup>
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write,
    ).await?;

    let transaction = conn.transaction().await?;

    let role = Role::retrieve_uid(&transaction, &role_uid)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.uids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let query = "\
        insert into group_roles (role_id, group_id) \
        select $1 as role_id, \
               groups.id as group_id \
        from groups \
        where groups.uid = any($2) \
        on conflict on constraint group_roles_pkey do nothing";
    let params: sql::ParamsArray<2> = [role.id.local(), &json.uids];

    transaction.execute(query, &params).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_id_groups(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::sec::roles::DropRoleGroup>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        Scope::SecRoles,
        Ability::Write
    ).await?;

    let transaction = conn.transaction().await?;

    let role = Role::retrieve_uid(&transaction, &role_uid)
        .await?
        .kind(ApiErrorKind::RoleNotFound)?;

    if json.uids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    transaction.execute(
        "\
        delete from group_roles \
        using groups \
        where role_id = $1 and \
              group_id = groups.id and \
              groups.uid = any($2)",
        &[role.id.local(), &json.uids]
    ).await?;

    transaction.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
