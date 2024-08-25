use std::fmt::Write;

use rfs_lib::ids;
use rfs_api::Validator;

use axum::Router;
use axum::http::StatusCode;
use axum::extract::{State, Query, Path};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use futures::TryStreamExt;
use serde::Deserialize;

use crate::error::{ApiResult, ApiError};
use crate::error::api::{ApiErrorKind, Detail, Context};
use crate::state::ArcShared;
use crate::sec::authn::{self, initiator, session};
use crate::sec::authz::permission;
use crate::sql;
use crate::user;
use crate::routing::query::PaginationQuery;
use crate::db;

mod group;
mod password;
mod totp;

#[derive(Deserialize)]
struct PathParams {
    user_uid: ids::UserUid
}

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .route("/", get(retrieve)
            .post(create))
        .route("/group", get(group::retrieve)
            .post(group::create))
        .route("/group/:group_uid", get(group::retrieve_id)
            .patch(group::update_id)
            .delete(group::delete_id))
        .route("/group/:group_uid/users", get(group::retrieve_users)
            .post(group::add_users)
            .delete(group::delete_users))
        .route("/password", post(password::update))
        .route("/totp", get(totp::retrieve)
            .post(totp::create)
            .patch(totp::update)
            .delete(totp::delete))
        .route("/totp/recovery", get(totp::retrieve_recovery)
            .post(totp::create_recovery))
        .route("/totp/recovery/:key_id", get(totp::retrieve_recovery_key)
            .patch(totp::update_recovery_key)
            .delete(totp::delete_recovery_key))
        .route("/:user_uid", get(retrieve_id)
            .patch(update_id)
            .delete(delete_id))
}

async fn retrieve(
    db::Conn(conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::UserUid>>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::User,
        permission::Ability::Read
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![&last_id, &limit];

        conn.query_raw(
            "\
            select uid, \
                   username \
            from users \
            where users.id > (\
                select users.id \
                from users \
                where users.uid = $1\
            ) \
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
            select uid, \
                   username \
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
            uid: row.get(0),
            username: row.get(1),
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

async fn create(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<rfs_api::users::CreateUser>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    state.sec().rbac().api_ability(
        &conn,
        &initiator,
        permission::Scope::User,
        permission::Ability::Write,
    ).await?;

    json.validate()?;

    let uid = ids::UserUid::gen();
    let username = json.username;

    let transaction = conn.transaction().await?;

    let email = if let Some(email) = json.email {
        let (username_id, email_id) = user::check_username_and_email(&transaction, &username, &email).await?;

        if username_id.is_some() {
            return Err(ApiError::from((
                ApiErrorKind::AlreadyExists,
                Detail::with_key("username")
            )));
        }

        if email_id.is_some() {
            return Err(ApiError::from((
                ApiErrorKind::AlreadyExists,
                Detail::with_key("email")
            )));
        }

        Some(email)
    } else {
        let username_id = user::check_username(&transaction, &username).await?;

        if username_id.is_some() {
            return Err(ApiError::from((
                ApiErrorKind::AlreadyExists,
                Detail::with_key("username")
            )));
        }

        None
    };

    let row = transaction.query_one(
        "\
        insert into users (uid, username, email) \
        values ($1, $2, $3) \
        returning id",
        &[&uid, &username, &email]
    ).await?;

    let id = row.get(0);

    authn::password::Password::create(&transaction, &id, json.password, state.sec().peppers()).await?;

    transaction.commit().await?;

    let email = email.map(|v| rfs_api::users::Email {
        email: v,
        verified: false
    });

    Ok((
        StatusCode::CREATED,
        rfs_api::Payload::new(rfs_api::users::User {
            uid,
            username,
            email
        })
    ))
}

async fn retrieve_id(
    db::Conn(conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { user_uid }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::User,
        permission::Ability::Read,
    ).await?;

    let user = user::User::retrieve_uid(&conn, &user_uid)
        .await?
        .kind(ApiErrorKind::UserNotFound)?;

    let email = user.email.map(|e| rfs_api::users::Email {
        email: e.email,
        verified: e.verified
    });

    Ok(rfs_api::Payload::new(rfs_api::users::User {
        uid: user.id.into(),
        username: user.username,
        email
    }))
}

async fn update_id(
    db::Conn(mut conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { user_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::users::UpdateUser>,
) -> ApiResult<impl IntoResponse> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::User,
        permission::Ability::Write,
    ).await?;

    let mut user = user::User::retrieve_uid(&conn, &user_uid)
        .await?
        .kind(ApiErrorKind::UserNotFound)?;

    if !json.has_work() {
        return Err(ApiError::api(ApiErrorKind::NoWork));
    }

    let transaction = conn.transaction().await?;

    {
        let mut use_comma = false;
        let mut update_query = String::from("update users set");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(user.id.local());

        if let Some(username) = json.username {
            use_comma = true;

            if !rfs_lib::users::username_valid(&username) {
                return Err(ApiError::from((
                    ApiErrorKind::ValidationFailed,
                    Detail::with_key("username")
                )));
            };

            if let Some(found_id) = user::check_username(&transaction, &username).await? {
                if found_id != *user.id.local() {
                    return Err(ApiError::from((
                        ApiErrorKind::AlreadyExists,
                        Detail::with_key("username")
                    )));
                }
            }

            user.username = username;

            write!(
                &mut update_query,
                " username = ${}",
                sql::push_param(&mut update_params, &user.username)
            ).unwrap();
        }

        if let Some(opt_email) = json.email {
            if use_comma {
                update_query.push(',');
            } else {
                //use_comma = true;
            }

            if let Some(email) = opt_email {
                if !rfs_lib::users::email_valid(&email) {
                    return Err(ApiError::from((
                        ApiErrorKind::ValidationFailed,
                        Detail::with_key("email")
                    )));
                };

                if let Some(found_id) = user::check_email(&transaction, &email).await? {
                    if found_id != *user.id.local() {
                        return Err(ApiError::from((
                            ApiErrorKind::AlreadyExists,
                            Detail::with_key("email")
                        )));
                    }
                }

                user.email = Some(user::UserEmail {
                    email,
                    verified: false
                });

                write!(
                    &mut update_query,
                    " email = ${}, email_verified = false",
                    sql::push_param(
                        &mut update_params,
                        &user.email.as_ref().unwrap().email
                    )
                ).unwrap();
            } else {
                update_query.push_str(
                    " email = null, email_verified = false"
                );
            }
        }

        write!(&mut update_query, " where id = $1").unwrap();

        transaction.execute(update_query.as_str(), update_params.as_slice()).await?;
    }

    let email = user.email.map(|e| rfs_api::users::Email {
        email: e.email,
        verified: e.verified
    });

    Ok(rfs_api::Payload::new(rfs_api::users::User {
        uid: user.id.into_uid(),
        username: user.username,
        email
    }))
}

async fn delete_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { user_uid }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let rbac = state.sec().rbac();

    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::User,
        permission::Ability::Write
    ).await?;

    let user = user::User::retrieve_uid(&conn, &user_uid)
        .await?
        .kind(ApiErrorKind::UserNotFound)?;

    if *user.id.uid() == user_uid {
        return Err(ApiError::from(ApiErrorKind::NoOp));
    }

    let transaction = conn.transaction().await?;

    let session = state.sec().session_info().cache();

    let session_tokens = session::Session::delete_user_sessions(
        &transaction,
        user.id.local(),
        None,
    ).await?;

    futures::pin_mut!(session_tokens);

    while let Some(token) = session_tokens.try_next().await? {
        session.remove(&token);
    }

    rbac.clear_id(user.id.local());

    Ok(StatusCode::NO_CONTENT)
}
