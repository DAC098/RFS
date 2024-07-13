use std::fmt::Write;
use std::path::PathBuf;

use rfs_lib::ids;
use rfs_api::fs::{CreateStorage, StorageMin, UpdateStorage};
use rfs_api::fs::backend::{CreateConfig, UpdateConfig};

use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{Context, Detail, ApiErrorKind};
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::fs;
use crate::tags;

#[derive(Deserialize)]
pub struct PathParams {
    storage_id: ids::StorageId,
}

pub async fn retrieve(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Read,
    ).await? {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    let result = conn.query_raw(
        "\
        select storage.id, \
               storage.name, \
               storage.user_id, \
               storage.backend, \
        from storage \
        where storage.user_id = $1 \
        order by storage.id",
        &[&initiator.user.id]
    ).await?;

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(10);

    while let Some(row) = result.try_next().await? {
        list.push(StorageMin {
            id: row.get(0),
            name: row.get(1),
            user_id: row.get(2),
            backend: sql::de_from_sql(row.get(3)),
        });
    }

    Ok(rfs_api::Payload::new(list))
}

pub async fn create(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<CreateStorage>,
) -> ApiResult<impl IntoResponse> {
    tracing::event!(
        tracing::Level::DEBUG,
        "creating new storage medium"
    );

    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await? {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    let backend = match json.backend {
        CreateConfig::Local { path } => {
            if !path.is_absolute() {
                return Err(ApiError::from(ApiErrorKind::NotAbsolutePath));
            }

            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::NotFound => {
                            return Err(ApiError::from(ApiErrorKind::DirNotFound));
                        },
                        _ => {
                            return Err(err.into())
                        }
                    }
                }
            };

            if !metadata.is_dir() {
                return Err(ApiError::from(ApiErrorKind::NotDirectory));
            }

            tokio::fs::create_dir_all(&path).await?;

            fs::backend::Config::Local(fs::backend::ConfigLocal { path })
        }
    };

    let transaction = conn.transaction().await?;

    let id = state.ids().wait_storage_id()?;
    let created = chrono::Utc::now();

    if !rfs_lib::fs::storage::name_valid(&json.name) {
        return Err(ApiError::from((
            ApiErrorKind::ValidationFailed,
            Detail::with_key("name")
        )));
    };

    if fs::Storage::name_check(&transaction, &json.name).await?.is_some() {
        return Err(ApiError::from((
            ApiErrorKind::AlreadyExists,
            Detail::with_key("name")
        )));
    }

    {
        let pg_backend = sql::ser_to_sql(&backend);

        transaction.execute(
            "\
            insert into storage (id, user_id, name, backend, created) values \
            ($1, $2, $3, $4, $5)",
            &[&id, initiator.user().id(), &json.name, &pg_backend, &created]
        ).await?;

        if !tags::validate_map(&json.tags) {
            return Err(ApiError::from(ApiErrorKind::InvalidTags));
        }

        tags::create_tags(&transaction, "storage_tags", "storage_id", &id, &json.tags).await?;
    }

    let storage = fs::Storage {
        id,
        name: json.name,
        user_id: initiator.user().id().clone(),
        backend,
        tags: json.tags,
        created,
        updated: None,
        deleted: None,
    };

    let created = chrono::Utc::now();
    let id = state.ids().wait_fs_id()?;
    let backend = fs::backend::Node::Local(fs::backend::NodeLocal {
        path: PathBuf::new()
    });

    {
        let pg_backend = sql::ser_to_sql(&backend);

        transaction.execute(
            "\
            insert into fs (id, user_id, storage_id, basename, fs_type, backend, created) values \
            ($1, $2, $3, $4, $5, $6, $7)",
            &[
                &id,
                &initiator.user.id,
                &storage.id,
                &storage.name,
                &fs::consts::ROOT_TYPE,
                &pg_backend,
                &created
            ]
        ).await?;
    }

    transaction.commit().await?;

    Ok((StatusCode::CREATED, rfs_api::Payload::new(storage.into_schema())))
}

pub async fn retrieve_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Read,
    ).await? {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    let storage = fs::Storage::retrieve(&conn, &storage_id)
        .await?
        .kind(ApiErrorKind::StorageNotFound)?;

    if storage.deleted.is_some() {
        return Err(ApiError::from(ApiErrorKind::StorageNotFound));
    }

    if storage.user_id != initiator.user.id {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    Ok(rfs_api::Payload::new(storage.into_schema()))
}

pub async fn update_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
    axum::Json(json): axum::Json<UpdateStorage>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await? {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    let mut storage = fs::Storage::retrieve(&conn, &storage_id)
        .await?
        .kind(ApiErrorKind::StorageNotFound)?;

    if storage.deleted.is_some() {
        return Err(ApiError::from(ApiErrorKind::StorageNotFound));
    }

    if storage.user_id != initiator.user.id {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    if !json.has_work() {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let transaction = conn.transaction().await?;

    if json.name.is_some() || json.backend.is_some() {
        let updated = chrono::Utc::now();
        let mut update_query = String::from("update storage set updated = $2");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&storage_id);
        update_params.push(&updated);

        if let Some(name) = json.name {
            if !rfs_lib::fs::storage::name_valid(&name) {
                return Err(ApiError::from((
                    ApiErrorKind::ValidationFailed,
                    Detail::with_key("name")
                )));
            };

            if let Some(found_id) = fs::Storage::name_check(&transaction, &name).await? {
                if found_id != storage_id {
                    return Err(ApiError::from((
                        ApiErrorKind::AlreadyExists,
                        Detail::with_key("name")
                    )));
                }
            }

            storage.name = name;

            write!(
                &mut update_query,
                "name = ${} ",
                sql::push_param(&mut update_params, &storage.name)
            ).unwrap();
        }

        if let Some(backend) = &json.backend {
            match backend {
                UpdateConfig::Local {..} => {}
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

        storage.tags = tags;
    }

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(storage.into_schema()))
}

pub async fn delete_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_id }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await? {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    let storage = fs::Storage::retrieve(&conn, &storage_id)
        .await?
        .kind(ApiErrorKind::StorageNotFound)?;

    if storage.user_id != initiator.user.id {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
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
