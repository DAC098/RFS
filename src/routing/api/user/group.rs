use rfs_lib::ids;

use axum::http::StatusCode;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use tokio_postgres::error::SqlState;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{ApiErrorKind, Detail, Context};
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::routing::query::PaginationQuery;
use crate::user;

#[derive(Deserialize)]
pub struct Params {
    pub group_uid: ids::GroupUid,
}

pub async fn retrieve(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::GroupUid>>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Read,
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![&last_id, &limit];

        conn.query_raw(
            "\
            select groups.uid, \
                   groups.name \
            from groups \
            where groups.id > (\
                select groups.id \
                from groups \
                where groups.uid = $1\
            ) \
            order by groups.id \
            limit $2",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![&limit, &offset_num];

        conn.query_raw(
            "\
            select groups.uid, \
                   groups.name \
            from groups \
            order by groups.id \
            limit $1 \
            offset $2",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::users::groups::ListItem {
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
    axum::Json(json): axum::Json<rfs_api::users::groups::CreateGroup>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Write
    ).await?;

    let uid = ids::GroupUid::gen();
    let name = json.name;
    let created = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    if let Err(err) = transaction.execute( "\
        insert into groups (uid, name, created) \
        values ($1, $2, $3) \
        returning id",
        &[&uid, &name, &created]
    ).await {
        let Some(db_error) = err.as_db_error() else {
            return Err(err.into());
        };

        if *db_error.code() == SqlState::UNIQUE_VIOLATION {
            let Some(constraint) = db_error.constraint() else {
                return Err(err.into());
            };

            if constraint == "groups_name_key" {
                return Err(ApiError::from((
                    ApiErrorKind::AlreadyExists,
                    Detail::with_key("name")
                )));
            }
        }

        return Err(err.into());
    }

    let rtn = rfs_api::users::groups::Group {
        uid,
        name,
        created,
        updated: None
    };

    transaction.commit().await?;

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(rtn)
    ))
}

pub async fn retrieve_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_uid }): Path<Params>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Read,
    ).await?;

    let group = user::group::Group::retrieve_uid(&conn, &group_uid)
        .await?
        .kind(ApiErrorKind::GroupNotFound)?;

    Ok(rfs_api::Payload::new(rfs_api::users::groups::Group {
        uid: group.id.into_uid(),
        name: group.name,
        created: group.created,
        updated: group.updated,
    }))
}

pub async fn update_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_uid }): Path<Params>,
    axum::Json(json): axum::Json<rfs_api::users::groups::UpdateGroup>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await?;

    let group = user::group::Group::retrieve_uid(&conn, &group_uid)
        .await?
        .kind(ApiErrorKind::GroupNotFound)?;

    let name = json.name;
    let updated = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    match transaction.execute(
        "\
        update groups \
        set name = $2 \
        where id = $1",
        &[group.id.local(), &name]
    ).await {
        Ok(_c) => {},
        Err(err) => {
            let Some(db_error) = err.as_db_error() else {
                return Err(err.into());
            };

            if *db_error.code() == SqlState::UNIQUE_VIOLATION {
                let Some(constraint) = db_error.constraint() else {
                    return Err(err.into());
                };

                if constraint == "groups_name_key" {
                    return Err(ApiError::from((
                        ApiErrorKind::AlreadyExists,
                        Detail::with_key("name")
                    )));
                }
            }

            return Err(err.into());
        }
    }

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(rfs_api::users::groups::Group {
        uid: group.id.into_uid(),
        name,
        created: group.created,
        updated: Some(updated),
    }))
}

pub async fn delete_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_uid }): Path<Params>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await?;

    let original = user::group::Group::retrieve_uid(&conn, &group_uid)
        .await?
        .kind(ApiErrorKind::GroupNotFound)?;

    let transaction = conn.transaction().await?;

    let group_users = transaction.query(
        "delete from group_users where group_id = $1 returning user_id",
        &[original.id.local()]
    ).await?;

    let _group = transaction.execute(
        "delete from groups where id = $1",
        &[original.id.local()]
    ).await?;

    transaction.commit().await?;

    let rbac = state.sec().rbac();

    for row in group_users {
        let id = row.get(0);

        rbac.clear_id(&id);
    }

    Ok(rfs_api::Payload::new(rfs_api::users::groups::Group {
        uid: original.id.into_uid(),
        name: original.name,
        created: original.created,
        updated: original.updated
    }))
}

pub async fn retrieve_users(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_uid }): Path<Params>,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::UserUid>>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Read
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);
    let offset_num = limit.sql_offset(offset);
    let params: sql::ParamsArray<3>;
    let query: &str;

    // this is probably a bad idea
    let maybe_last_id;

    if let Some(last_id) = last_id {
        maybe_last_id = last_id;

        params = [&group_uid, &maybe_last_id, &limit];
        query = "\
            select users.uid \
            from users \
            join group_users on \
                users.id = group_users.user_id \
            join groups on \
                group_user.group_id = groups.id \
            where groups.uid = $1 and \
                  users.id > (\
                      select users.id \
                      from users \
                      where users.uid = $2) \
            order by users.id \
            limit $3";
    } else {
        pagination.set_offset(offset);

        params = [&group_uid, &limit, &offset_num];
        query = "\
            select users.uid \
            from users \
            join group_users on \
                users.id = group_users.user_id \
            join groups on \
                group_users.group_id = groups.id \
            where groups.uid = $1 \
            order by users.id \
            limit $2 \
            offset $3";
    }

    let group_fut = user::group::Group::retrieve_uid(&conn, &group_uid);
    let users_fut = conn.query_raw(query, params);

    let result = match tokio::try_join!(group_fut, users_fut) {
        Ok((Some(_), rows)) => rows,
        Ok((None, _)) => {
            return Err(ApiError::from(ApiErrorKind::GroupNotFound));
        },
        Err(err) => {
            return Err(ApiError::from(err));
        }
    };

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::users::groups::GroupUser {
            uid: row.get(0),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn add_users(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_uid }): Path<Params>,
    axum::Json(json): axum::Json<rfs_api::users::groups::AddUsers>
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await?;

    let group = user::group::Group::retrieve_uid(&conn, &group_uid)
        .await?
        .kind(ApiErrorKind::GroupNotFound)?;

    if json.uids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let transaction = conn.transaction().await?;

    let params: sql::ParamsArray<2> = [group.id.local(), &json.uids];
    let result = transaction.query_raw(
        "\
        insert into group_users (group_id, user_id)
        select $1 as group_id, \
               users.id as user_id \
        from users \
        where users.uid = any($2) \
        on conflict on constraint group_users_pkey do nothing \
        returning users.id",
        params
    ).await?;

    transaction.commit().await?;

    futures::pin_mut!(result);

    let rbac = state.sec().rbac();

    while let Some(row) = result.try_next().await? {
        let user_id = row.get(0);

        rbac.clear_id(&user_id);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_users(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(Params { group_uid }): Path<Params>,
    axum::Json(json): axum::Json<rfs_api::users::groups::DropUsers>
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::UserGroup,
        permission::Ability::Write,
    ).await?;

    let group = user::group::Group::retrieve_uid(&conn, &group_uid)
        .await?
        .kind(ApiErrorKind::GroupNotFound)?;

    if json.uids.len() == 0 {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let transaction = conn.transaction().await?;

    let params: sql::ParamsArray<2> = [group.id.local(), &json.uids];
    let result = transaction.query_raw(
        "\
        delete from group_users \
        using users \
        where group_users.user_id = users.id and \
              users.uid <> all($2) and \
              group_id = $1",
        params
    ).await?;

    transaction.commit().await?;

    futures::pin_mut!(result);

    let rbac = state.sec().rbac();

    while let Some(row) = result.try_next().await? {
        let user_id = row.get(0);

        rbac.clear_id(&user_id);
    }

    Ok(StatusCode::NO_CONTENT)
}
