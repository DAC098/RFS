use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;
use tokio_postgres::error::SqlState;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;

pub mod group_id;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::UserGroup,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let params: sql::ParamsVec = vec![];

    let result = conn.query_raw(
        "select id, name from groups",
        params
    ).await?;

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        let item = rfs_api::users::groups::ListItem {
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
    axum::Json(json): axum::Json<rfs_api::users::groups::CreateGroup>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::UserGroup,
        permission::Ability::Write
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let name = json.name;
    let created = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    let result = match transaction.query_one( "\
        insert into groups (name, created) \
        values ($1, $2) \
        returning id",
        &[&name, &created]
    ).await {
        Ok(r) => r,
        Err(err) => {
            let Some(db_error) = err.as_db_error() else {
                return Err(err.into());
            };

            if *db_error.code() == SqlState::UNIQUE_VIOLATION {
                let Some(constraint) = db_error.constraint() else {
                    return Err(err.into());
                };

                if constraint == "groups_name_key" {
                    return Err(error::Error::api((
                        error::ApiErrorKind::AlreadyExists,
                        error::Detail::with_key("name")
                    )));
                }
            }

            return Err(err.into());
        }
    };

    let rtn = rfs_api::users::groups::Group {
        id: result.get(0),
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
