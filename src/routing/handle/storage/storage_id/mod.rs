use std::fmt::Write;

use rfs_lib::ids;
use rfs_api::fs::storage::{UpdateStorage, UpdateStorageType};

use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::storage;
use crate::tags;

#[derive(Deserialize)]
pub struct PathParams {
    storage_id: ids::StorageId,
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Read,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(medium) = storage::Medium::retrieve(
        &conn,
        &storage_id
    ).await? else {
        return Err(error::Error::api(error::ApiErrorKind::StorageNotFound));
    };

    if medium.deleted.is_some() {
        return Err(error::Error::api(error::ApiErrorKind::StorageNotFound));
    }

    if medium.user_id != *initiator.user().id() {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    Ok(rfs_api::Payload::new(medium.into_schema()))
}

pub async fn put(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
    axum::Json(json): axum::Json<UpdateStorage>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(mut medium) = storage::Medium::retrieve(
        &conn,
        &storage_id
    ).await? else {
        return Err(error::Error::api(error::ApiErrorKind::StorageNotFound));
    };

    if medium.deleted.is_some() {
        return Err(error::Error::api(error::ApiErrorKind::StorageNotFound));
    }

    if medium.user_id != *initiator.user().id() {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    if !json.has_work() {
        return Err(error::Error::api(error::ApiErrorKind::NoWork));
    }

    let transaction = conn.transaction().await?;

    if json.name.is_some() || json.type_.is_some() {
        let updated = chrono::Utc::now();
        let mut update_query = String::from("update storage set updated = $2");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&storage_id);
        update_params.push(&updated);

        if let Some(name) = json.name {
            if !rfs_lib::storage::name_valid(&name) {
                return Err(error::Error::api((
                    error::ApiErrorKind::ValidationFailed,
                    error::Detail::with_key("name")
                )));
            };

            if let Some(found_id) = storage::name_check(&transaction, &initiator.user.id, &name).await? {
                if found_id != storage_id {
                    return Err(error::Error::api((
                        error::ApiErrorKind::AlreadyExists,
                        error::Detail::with_key("name")
                    )));
                }
            }

            medium.name = name;

            write!(
                &mut update_query,
                "name = ${} ",
                sql::push_param(&mut update_params, &medium.name)
            ).unwrap();
        }

        if let Some(type_) = &json.type_ {
            match type_ {
                UpdateStorageType::Local {..} => {}
            }
        }

        write!(&mut update_query, "where storage_id = $1").unwrap();

        transaction.execute(update_query.as_str(), update_params.as_slice()).await?;
    }

    if let Some(tags) = json.tags {
        tags::update_tags(
            &transaction,
            "storage_tags",
            "storage_id",
            &storage_id,
            &tags
        ).await?;

        medium.tags = tags;
    }

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(medium.into_schema()))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let Some(medium) = storage::Medium::retrieve(
        &conn,
        &storage_id
    ).await? else {
        return Err(error::Error::api(error::ApiErrorKind::StorageNotFound));
    };

    if medium.user_id != initiator.user.id {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let deleted = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    // soft delete fs items
    let _ = transaction.execute(
        "update fs set deleted = $2 where storage_id = $1",
        &[&storage_id, &deleted]
    ).await?;

    // soft delete storage item
    let _ = transaction.execute(
        "update storage set deleted = $2 where storage_id = $1",
        &[&storage_id, &deleted]
    ).await?;

    Ok(StatusCode::OK)
}
