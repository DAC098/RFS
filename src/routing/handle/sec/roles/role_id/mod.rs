use std::collections::HashSet;
use std::fmt::Write;

use rfs_lib::{schema, actions};

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission::{has_ability, Role, Permission, Ability, Scope};
use crate::sql;

pub mod users;
pub mod groups;

#[derive(Deserialize)]
pub struct PathParams {
    role_id: i64
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !has_ability(
        &conn,
        initiator.user().id(),
        Scope::SecRoles,
        Ability::Read,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let role_params: sql::ParamsArray<1> = [&role_id];

    let (role_result, permissions_result) = match tokio::try_join!(
        conn.query_opt(
            "select id, name from authz_roles where id = $1",
            &role_params
        ),
        conn.query_raw(
            "select role_id, scope, ability from authz_permissions where role_id = $1",
            [&role_id]
        )
    ) {
        Ok((Some(role), permissions)) => (role, permissions),
        Ok((None, _)) => {
            return Err(error::Error::api(error::SecKind::RoleNotFound));
        },
        Err(err) => {
            return Err(err.into());
        }
    };

    let mut permissions = Vec::new();

    futures::pin_mut!(permissions_result);

    while let Some(row) = permissions_result.try_next().await? {
        permissions.push(schema::sec::Permission {
            scope: row.get(1),
            ability: row.get(2),
        });
    }

    let rtn = schema::sec::Role {
        id: role_result.get(0),
        name: role_result.get(1),
        permissions,
    };

    Ok(net::Json::new(rfs_lib::json::Wrapper::new(rtn)))
}

pub async fn patch(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
    axum::Json(json): axum::Json<actions::sec::UpdateRole>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !has_ability(
        &conn,
        initiator.user().id(),
        Scope::SecRoles,
        Ability::Write
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied))
    }

    let transaction = conn.transaction().await?;

    let Some(original) = Role::retrieve(&transaction, &role_id).await? else {
        return Err(error::Error::api(error::SecKind::RoleNotFound));
    };

    if json.name.is_none() && json.permissions.is_none() {
        return Err(error::Error::api(error::GeneralKind::NoWork));
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
                        return Err(error::Error::api((
                            error::GeneralKind::AlreadyExists,
                            error::Detail::with_key("name")
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

    let permissions = if let Some(changed) = changed_permissions {
        changed.into_iter()
            .map(|(scope, ability)| schema::sec::Permission {
                scope,
                ability
            })
            .collect()
    } else {
        Permission::retrieve_by_role_id(&conn, &role_id)
            .await?
            .into_iter()
            .map(|perm| schema::sec::Permission {
                scope: perm.scope,
                ability: perm.ability
            })
            .collect()
    };

    let wrapper = rfs_lib::json::Wrapper::new(schema::sec::Role {
        id: role_id,
        name,
        permissions,
    });

    Ok(net::Json::new(wrapper))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { role_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !has_ability(
        &conn,
        initiator.user().id(),
        Scope::SecRoles,
        Ability::Write
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let transaction = conn.transaction().await?;

    let Some(_original) = Role::retrieve(&transaction, &role_id).await? else {
        return Err(error::Error::api(error::SecKind::RoleNotFound));
    };

    let query_params: sql::ParamsArray<1> = [&role_id];

    let result = tokio::try_join!(
        transaction.execute(
            "delete from authz_permissions where role_id $1",
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
        "delete from authz_roles where id = $!",
        &query_params
    ).await?;

    transaction.commit().await?;

    Ok(net::Json::empty())
}
