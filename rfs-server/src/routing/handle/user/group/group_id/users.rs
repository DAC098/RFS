use std::fmt::Write;

use rfs_lib::{ids, schema, actions};

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::user;

#[derive(Deserialize)]
pub struct Params {
    pub group_id: ids::GroupId,
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::UserGroup,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let _params: sql::ParamsVec = vec![&group_id];

    let Some(_group) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::api(error::UserKind::GroupNotFound));
    };

    let result = conn.query_raw(
        "\
        select users.id \
        from users \
        join group_users on \
            users.id = group_users.user_id \
        where group_users.group_id = $1",
        &[&group_id]
    ).await?;

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = schema::user::group::GroupUser {
            id: row.get(0),
        };

        list.push(item);
    }

    let wrapper = rfs_lib::json::ListWrapper::with_vec(list);

    Ok(net::Json::new(wrapper))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
    axum::Json(json): axum::Json<actions::user::group::AddUsers>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let Some(_group) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::api(error::UserKind::GroupNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::GeneralKind::NoWork));
    }

    let mut id_iter = json.ids.iter();
    let mut query = String::from(
        "\
        insert into group_users (group_id, user_id) \
        values"
    );
    let mut params: sql::ParamsVec = Vec::with_capacity(json.ids.len() + 1);
    params.push(&group_id);

    if let Some(first) = id_iter.next() {
        write!(&mut query, " ($1, ${})", sql::push_param(&mut params, first))?;

        while let Some(id) = id_iter.next() {
            write!(&mut query, ", ($1, ${})", sql::push_param(&mut params, id))?;
        }
    }

    write!(&mut query, " on conflict on constraint group_users_pkey do nothing")?;

    let transaction = conn.transaction().await?;

    let _ = transaction.execute(query.as_str(), params.as_slice()).await?;

    transaction.commit().await?;

    Ok(net::Json::empty())
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
    axum::Json(json): axum::Json<actions::user::group::DropUsers>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let Some(_group) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::api(error::UserKind::GroupNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::GeneralKind::NoWork));
    }

    let transaction = conn.transaction().await?;

    let _ = transaction.execute(
        "\
        delete from group_users \
        where group_id = $1 and \
              user_id <> all($2)",
        &[&group_id, &json.ids]
    ).await?;

    transaction.commit().await?;

    Ok(net::Json::empty())
}
