use std::fmt::Write;

use rfs_lib::{ids, schema, actions};

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::user;

#[derive(Deserialize)]
pub struct PathParams {
    user_id: ids::UserId
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { user_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::User,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let Some(user) = user::User::retrieve(&conn, &user_id).await? else {
        return Err(error::Error::api(error::UserKind::NotFound));
    };

    let email = user.email.map(|e| schema::user::Email {
        email: e.email,
        verified: e.verified
    });
    let rtn = rfs_lib::json::Wrapper::new(schema::user::User {
        id: user.id,
        username: user.username,
        email
    });

    Ok(net::Json::new(rtn))
}

pub async fn patch(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { user_id }): Path<PathParams>,
    axum::Json(json): axum::Json<actions::user::UpdateUser>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::User,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let Some(mut user) = user::User::retrieve(&conn, &user_id).await? else {
        return Err(error::Error::api(error::UserKind::NotFound));
    };

    if !json.has_work() {
        return Err(error::Error::api(error::GeneralKind::NoWork));
    }

    let transaction = conn.transaction().await?;

    {
        let mut use_comma = false;
        let mut update_query = String::from("update users set");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&user_id);

        if let Some(username) = json.username {
            use_comma = true;

            if !rfs_lib::user::username_valid(&username) {
                return Err(error::Error::api((
                    error::GeneralKind::ValidationFailed,
                    error::Detail::with_key("username")
                )));
            };

            if let Some(found_id) = user::check_username(&transaction, &username).await? {
                if found_id != user_id {
                    return Err(error::Error::api((
                        error::GeneralKind::AlreadyExists,
                        error::Detail::with_key("username")
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
                if !rfs_lib::user::email_valid(&email) {
                    return Err(error::Error::api((
                        error::GeneralKind::ValidationFailed,
                        error::Detail::with_key("email")
                    )));
                };

                if let Some(found_id) = user::check_email(&transaction, &email).await? {
                    if found_id != user_id {
                        return Err(error::Error::api((
                            error::GeneralKind::AlreadyExists,
                            error::Detail::with_key("email")
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

    let email = user.email.map(|e| schema::user::Email {
        email: e.email,
        verified: e.verified
    });
    let rtn = rfs_lib::json::Wrapper::new(schema::user::User {
        id: user.id,
        username: user.username,
        email
    });

    Ok(net::Json::new(rtn))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { user_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        initiator.user().id(),
        permission::Scope::User,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::AuthKind::PermissionDenied));
    }

    let Some(user) = user::User::retrieve(&conn, &user_id).await? else {
        return Err(error::Error::api(error::UserKind::NotFound));
    };

    if user.id == user_id {
        return Err(error::Error::api(error::GeneralKind::Noop));
    }

    // this will need to be decided along with the fs and storage delete update
    Ok(net::Json::empty())
}
