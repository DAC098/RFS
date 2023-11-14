use std::collections::HashSet;
use std::fmt::Write;

use rfs_lib::{ids, schema, actions};
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;
use tokio_postgres::error::SqlState;

use crate::net::{self, error};
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
        initiator.user().id(),
        Scope::SecRoles,
        Ability::Read,
    ).await? {
        return Err(error::Error::new()
            .status(StatusCode::UNAUTHORIZED)
            .kind("PermissionDenied"));
    }

    let params: sql::ParamsVec = vec![];

    let result = conn.query_raw(
        "select id, name from authz_roles",
        params
    ).await?;

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = schema::sec::RoleListItem {
            id: row.get(0),
            name: row.get(1),
        };

        list.push(item);
    }

    let wrapper = rfs_lib::json::ListWrapper::with_vec(list);

    Ok(net::Json::new(wrapper))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<actions::sec::CreateRole>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        Scope::SecRoles,
        Ability::Write
    ).await? {
        return Err(error::Error::new()
            .status(StatusCode::UNAUTHORIZED)
            .kind("PermissionDenied"));
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
                    return Err(error::Error::new()
                        .status(StatusCode::BAD_REQUEST)
                        .kind("RoleNameExists")
                        .message("requested role name already exists"));
                }
            }

            return Err(err.into());
        }
    };

    let role_id: ids::RoleId = result.get(0);

    if json.permissions.len() == 0 {
        transaction.commit().await?;

        let wrapper = rfs_lib::json::Wrapper::new(schema::sec::Role {
            id: role_id,
            name: json.name,
            permissions: Vec::new(),
        });

        return Ok(net::Json::new(wrapper));
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
        .map(|(ability,scope)| schema::sec::Permission {
            scope,
            ability
        })
        .collect();

    let wrapper = rfs_lib::json::Wrapper::new(schema::sec::Role {
        id: role_id,
        name: json.name,
        permissions
    });

    Ok(net::Json::new(wrapper))
}
