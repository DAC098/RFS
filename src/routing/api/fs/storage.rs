use std::fmt::Write;
use std::path::PathBuf;

use rfs_api::fs::{CreateStorage, StorageMin, UpdateStorage};
use rfs_api::fs::backend::{CreateConfig, UpdateConfig};
use rfs_lib::ids;

use axum::extract::{Path, State, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures::TryStreamExt;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{Context, Detail, ApiErrorKind};
use crate::fs;
use crate::routing::query::PaginationQuery;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::state::ArcShared;
use crate::tags;

#[derive(Deserialize)]
pub struct PathParams {
    storage_uid: ids::StorageUid,
}

pub async fn retrieve(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::StorageUid>>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Storage,
        permission::Ability::Read,
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsArray<3> = [initiator.user.id.local(), &last_id, &limit];

        conn.query_raw(
            "\
            select storage.uid, \
                   storage.name, \
                   users.uid, \
                   storage.backend \
            from storage \
                join users on storage.user_id = users.id \
            where storage.user_id = $1 and \
                  storage.id > (\
                      select storage.id \
                      from storage \
                      where storage.uid = $2\
                  ) \
            order by storage.id \
            limit $3",
            params,
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsArray<3> = [initiator.user.id.local(), &limit, &offset_num];

        conn.query_raw(
            "\
            select storage.uid, \
                   storage.name, \
                   users.uid, \
                   storage.backend \
            from storage \
                join users on storage.user_id = users.id \
            where storage.user_id = $1 \
            order by storage.id \
            limit $2 \
            offset $3",
            params,
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::new();

    while let Some(row) = result.try_next().await? {
        list.push(StorageMin {
            uid: row.get(0),
            name: row.get(1),
            user_uid: row.get(2),
            backend: sql::de_from_sql(row.get(3)),
        });
    }

    Ok(rfs_api::Payload::from((pagination, list)))
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

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await?;

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

    let uid = ids::StorageUid::gen();
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

    let id = {
        let pg_backend = sql::ser_to_sql(&backend);

        let result = transaction.query_one(
            "\
            insert into storage (uid, user_id, name, backend, created) values \
            ($1, $2, $3, $4, $5) \
            returning id",
            &[&uid, initiator.user.id.local(), &json.name, &pg_backend, &created]
        ).await?;

        result.get(0)
    };

    if !tags::validate_map(&json.tags) {
        return Err(ApiError::from(ApiErrorKind::InvalidTags));
    }

    tags::create_tags(&transaction, "storage_tags", "storage_id", &id, &json.tags).await?;

    let storage = fs::Storage {
        id: ids::StorageSet::new(id, uid),
        name: json.name,
        user: initiator.user.id.clone(),
        backend,
        tags: json.tags,
        created,
        updated: None,
        deleted: None,
    };

    let created = chrono::Utc::now();
    let uid = ids::FSUid::gen();
    let backend = fs::backend::Node::Local(fs::backend::NodeLocal {
        path: PathBuf::new()
    });

    {
        let pg_backend = sql::ser_to_sql(&backend);

        transaction.execute(
            "\
            insert into fs (uid, user_id, storage_id, basename, fs_type, backend, created) values \
            ($1, $2, $3, $4, $5, $6, $7)",
            &[
                &uid,
                initiator.user.id.local(),
                storage.id.local(),
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
    Path(PathParams { storage_uid }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    let conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Storage,
        permission::Ability::Read,
    ).await?;

    let storage = fs::Storage::retrieve_uid(&conn, &storage_uid)
        .await?
        .kind(ApiErrorKind::StorageNotFound)?;

    if storage.deleted.is_some() {
        return Err(ApiError::from(ApiErrorKind::StorageNotFound));
    }

    if storage.user != initiator.user.id {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    Ok(rfs_api::Payload::new(storage.into_schema()))
}

pub async fn update_id(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { storage_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<UpdateStorage>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await?;

    let mut storage = fs::Storage::retrieve_uid(&conn, &storage_uid)
        .await?
        .kind(ApiErrorKind::StorageNotFound)?;

    if storage.deleted.is_some() {
        return Err(ApiError::from(ApiErrorKind::StorageNotFound));
    }

    if storage.user != initiator.user.id {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    if !json.has_work() {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let transaction = conn.transaction().await?;
    let local_id = *storage.id.local();

    if json.name.is_some() || json.backend.is_some() {
        let updated = chrono::Utc::now();
        let mut update_query = String::from("update storage set updated = $2");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&local_id);
        update_params.push(&updated);

        if let Some(name) = json.name {
            if !rfs_lib::fs::storage::name_valid(&name) {
                return Err(ApiError::from((
                    ApiErrorKind::ValidationFailed,
                    Detail::with_key("name")
                )));
            };

            if let Some(found_id) = fs::Storage::name_check(&transaction, &name).await? {
                if found_id != local_id {
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
            &local_id,
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
    Path(PathParams { storage_uid }): Path<PathParams>,
) -> ApiResult<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    permission::api_ability(
        &conn,
        &initiator,
        permission::Scope::Storage,
        permission::Ability::Write,
    ).await?;

    let storage = fs::Storage::retrieve_uid(&conn, &storage_uid)
        .await?
        .kind(ApiErrorKind::StorageNotFound)?;

    if storage.user != initiator.user.id {
        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
    }

    let deleted = chrono::Utc::now();

    let transaction = conn.transaction().await?;

    // soft delete fs items
    let _ = transaction.execute(
        "update fs set deleted = $2 where storage_id = $1",
        &[storage.id.local(), &deleted]
    ).await?;

    // soft delete storage item
    let _ = transaction.execute(
        "update storage set deleted = $2 where storage_id = $1",
        &[storage.id.local(), &deleted]
    ).await?;

    Ok(StatusCode::OK)
}
