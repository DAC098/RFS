use rfs_lib::ids::UserId;
use rfs_lib::query::{Limit, Offset};
use rfs_api::Validator;

use axum::http::StatusCode;
use axum::extract::{State, Query};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::{initiator, password};
use crate::sec::authz::permission;
use crate::sql;
use crate::user;

pub mod group;
pub mod user_id;

#[derive(Deserialize)]
pub struct GetQuery {
    #[serde(default)]
    limit: Limit,

    #[serde(default)]
    offset: Offset,

    last_id: Option<UserId>,
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
        permission::Scope::User,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![&last_id, &limit];

        conn.query_raw(
            "\
            select id, \
                   username, \
                   email, \
                   email_verified \
            from users \
            where users.id > $1 \
            order by users.id \
            limit $2",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![&limit, &offset_num];

        conn.query_raw(
            "\
            select id, \
                   username, \
                   email, \
                   email_verified \
            from users \
            order by users.id \
            limit $1 \
            offset $2",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(limit as usize);

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::users::ListItem {
            id: row.get(0),
            username: row.get(1),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<rfs_api::users::CreateUser>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::User,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    json.validate()?;

    let id = state.ids().wait_user_id()?;
    let username = json.username;

    let transaction = conn.transaction().await?;

    let email = if let Some(email) = json.email {
        let (username_id, email_id) = user::check_username_and_email(&transaction, &username, &email).await?;

        if username_id.is_some() {
            return Err(error::Error::api((
                error::ApiErrorKind::AlreadyExists,
                error::Detail::with_key("username")
            )));
        }

        if email_id.is_some() {
            return Err(error::Error::api((
                error::ApiErrorKind::AlreadyExists,
                error::Detail::with_key("email")
            )));
        }

        Some(email)
    } else {
        let username_id = user::check_username(&transaction, &username).await?;

        if username_id.is_some() {
            return Err(error::Error::api((
                error::ApiErrorKind::AlreadyExists,
                error::Detail::with_key("username")
            )));
        }

        None
    };

    transaction.execute(
        "insert into users (id, username, email) values ($1, $2, $3)",
        &[&id, &username, &email]
    ).await?;

    let _ = password::Password::create(&transaction, &id, json.password, state.sec().peppers()).await?;

    transaction.commit().await?;

    let email = email.map(|v| rfs_api::users::Email {
        email: v,
        verified: false
    });

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(rfs_api::users::User {
            id,
            username,
            email
        })
    ))
}
