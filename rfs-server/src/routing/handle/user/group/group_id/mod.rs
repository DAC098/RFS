use rfs_lib::{ids, schema, actions};
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use tokio_postgres::error::SqlState;
use serde::{Deserialize};

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sql;
use crate::user;

pub mod users;

#[derive(Deserialize)]
pub struct Params {
    pub group_id: ids::GroupId,
}

pub async fn get(
    State(state): State<ArcShared>,
    _initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;
    let params: sql::ParamsVec = vec![&group_id];

    let Some(group) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("GroupNotFound")
            .message("the requested group was not found"));
    };

    let wrapper = rfs_lib::json::Wrapper::new(schema::user::group::Group {
        id: group.id,
        name: group.name,
        created: group.created,
        updated: group.updated,
    });

    Ok(net::Json::new(wrapper))
}

pub async fn patch(
    State(state): State<ArcShared>,
    _initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
    axum::Json(json): axum::Json<actions::user::group::UpdateGroup>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(original) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("GroupNotFound")
            .message("the requested group was not found"));
    };

    let name = json.name;
    let updated = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    match transaction.execute(
        "\
        update groups \
        set name = $2 \
        where id = $1",
        &[&group_id, &name]
    ).await {
        Ok(c) => {},
        Err(err) => {
            let Some(db_error) = err.as_db_error() else {
                return Err(err.into());
            };

            if *db_error.code() == SqlState::UNIQUE_VIOLATION {
                let Some(constraint) = db_error.constraint() else {
                    return Err(err.into());
                };

                if constraint == "groups_name_key" {
                    return Err(error::Error::new()
                        .kind("GroupNameExists")
                        .message("requested group name already exists"));
                }
            }

            return Err(err.into());
        }
    }

    transaction.commit().await?;

    let rtn = rfs_lib::json::Wrapper::new(schema::user::group::Group {
        id: group_id,
        name,
        created: original.created,
        updated: Some(updated),
    });

    Ok(net::Json::new(rtn))
}

pub async fn delete(
    State(state): State<ArcShared>,
    _initiator: initiator::Initiator,
    Path(Params { group_id }): Path<Params>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let Some(original) = user::group::Group::retrieve(&conn, &group_id).await? else {
        return Err(error::Error::new()
            .status(StatusCode::NOT_FOUND)
            .kind("GroupNotFound")
            .message("the requested group was not found"));
    };

    let transaction = conn.transaction().await?;

    let _group_users = transaction.execute(
        "delete from group_users where group_id = $1",
        &[&group_id]
    ).await?;

    let _group = transaction.execute(
        "delete from groups where id = $1",
        &[&group_id]
    ).await?;

    transaction.commit().await?;

    let rtn = rfs_lib::json::Wrapper::new(schema::user::group::Group {
        id: original.id,
        name: original.name,
        created: original.created,
        updated: original.updated
    });

    Ok(net::Json::new(rtn))
}
