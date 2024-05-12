use std::collections::HashSet;
use std::fmt::Write;

use rfs_lib::ids;
use rfs_lib::query::{Limit, Offset};

use axum::http::StatusCode;
use axum::extract::{State, Query};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission::{self, Ability, Scope};
use crate::sql;

pub mod role_id;

#[derive(Deserialize)]
pub struct GetQuery {
    #[serde(default)]
    limit: Limit,

    #[serde(default)]
    offset: Offset,

    last_id: Option<ids::RoleId>,
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
        Scope::SecRoles,
        Ability::Read,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

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
