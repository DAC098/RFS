use std::collections::HashSet;
use std::fmt::Write;

use rfs_lib::ids;

use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;


use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission::{self, Ability, Scope};
use crate::sql;

pub mod role_id;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        Scope::SecRoles,
        Ability::Read,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let params: sql::ParamsVec = vec![];

    let result = conn.query_raw(
        "select id, name from authz_roles",
        params
    ).await?;

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::sec::roles::RoleListItem {
            id: row.get(0),
            name: row.get(1),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::new(list))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<rfs_api::sec::roles::CreateRole>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        Scope::SecRoles,
        Ability::Write
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

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
                    return Err(error::Error::api((
                        error::ApiErrorKind::AlreadyExists,
                        error::Detail::with_key("name")
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
