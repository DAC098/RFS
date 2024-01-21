use std::fmt::Write;

use rfs_lib::ids;
use rfs_lib::query::{Limit, Offset};

use axum::http::StatusCode;
use axum::extract::{Query, Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::user;

#[derive(Deserialize)]
pub struct Params {
    pub group_id: ids::GroupId,
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
    Path(Params { group_id }): Path<Params>,
    Query(GetQuery { limit, offset, last_id }): Query<GetQuery>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::UserGroup,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let mut pagination = rfs_api::Pagination::from(&limit);
    let offset_num = limit.sql_offset(offset);
    let params: sql::ParamsArray<3>;
    let query: &str;

    // this is probably a bad idea
    let maybe_last_id;

    if let Some(last_id) = last_id {
        maybe_last_id = last_id;

        params = [&group_id, &maybe_last_id, &limit];
        query = "\
            select users.id \
            from users \
            join group_users on \
                users.id = group_users.user_id \
            where group_users.group_id = $1 and \
                  users.id > $2 \
            order by users.id \
            limit $3";
    } else {
        pagination.set_offset(offset);

        params = [&group_id, &limit, &offset_num];
        query = "\
            select users.id \
            from users \
            join group_users on \
                users.id = group_users.user_id \
            where group_users.group_id = $1 \
            order by users.id \
            limit $2 \
            offset $3";
    }

    let group_fut = user::group::Group::retrieve(&conn, &group_id);
    let users_fut = conn.query_raw(query, params);

    let result = match tokio::try_join!(group_fut, users_fut) {
        Ok((Some(_), rows)) => rows,
        Ok((None, _)) => {
            return Err(error::Error::api(error::ApiErrorKind::GroupNotFound));
        },
        Err(err) => {
            return Err(error::Error::from(err));
        }
    };

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::users::groups::GroupUser {
            id: row.get(0),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
    axum::Json(json): axum::Json<rfs_api::users::groups::AddUsers>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(_group) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::GroupNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::ApiErrorKind::NoWork));
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

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
    axum::Json(json): axum::Json<rfs_api::users::groups::DropUsers>
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(_group) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::api(error::ApiErrorKind::GroupNotFound));
    };

    if json.ids.len() == 0 {
        return Err(error::Error::api(error::ApiErrorKind::NoWork));
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

    Ok(StatusCode::NO_CONTENT)
}
