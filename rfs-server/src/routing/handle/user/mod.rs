use rfs_lib::{schema, actions};
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;

use crate::net;
use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sql;
use crate::user;

pub mod group;
pub mod user_id;

pub async fn get(
    State(state): State<ArcShared>,
    _initiator: initiator::Initiator
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;
    let params: sql::ParamsVec = vec![];

    let result = conn.query_raw(
        "\
        select id, \
               username, \
               email, \
               email_verified \
        from users",
        params
    ).await?;

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(10);

    while let Some(row) = result.try_next().await? {
        let item = schema::user::ListItem {
            id: row.get(0),
            username: row.get(1),
        };

        list.push(item);
    }

    let wrapper = rfs_lib::json::ListWrapper::with_vec(list);

    Ok(net::Json::new(wrapper))
}

pub async fn post(
    State(state): State<ArcShared>,
    _initiator: initiator::Initiator,
    axum::Json(json): axum::Json<actions::user::CreateUser>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let id = state.ids().wait_user_id()?;

    let username = json.username;

    if !rfs_lib::user::username_valid(&username) {
        return Err(error::Error::new()
            .status(StatusCode::BAD_REQUEST)
            .kind("InvalidUsername")
            .message("the requested username is invalid"));
    };

    let email = if let Some(email) = json.email {
        if !rfs_lib::user::email_valid(&email) {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("InvalidEmail")
                .message("the requested email is invalid"));
        };

        let (username_id, email_id) = user::check_username_and_email(&conn, &username, &email).await?;

        if username_id.is_some() {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("UsernameExists")
                .message("the requested username is already in use"));
        }

        if email_id.is_some() {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("EmailExists")
                .message("the requested email is already in use"));
        }

        Some(email)
    } else {
        let username_id = user::check_username(&conn, &username).await?;

        if username_id.is_some() {
            return Err(error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("UsernameExists")
                .message("the requested username is already in use"));
        }

        None
    };

    let transaction = conn.transaction().await?;

    transaction.execute(
        "insert into users (id, username, email) values ($1, $2, $3)",
        &[&id, &username, &email]
    ).await?;

    transaction.commit().await?;

    let email = email.map(|v| schema::user::Email {
        email: v,
        verified: false
    });

    let rtn = rfs_lib::json::Wrapper::new(schema::user::User {
        id,
        username,
        email
    });

    Ok(net::Json::new(rtn))
}
