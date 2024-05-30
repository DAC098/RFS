use std::path::PathBuf;

use rfs_api::fs::{StorageMin, CreateStorage};
use rfs_api::fs::backend::CreateConfig;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::fs;
use crate::tags;

pub mod storage_id;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
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

pub async fn post(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    axum::Json(json): axum::Json<CreateStorage>,
) -> error::Result<impl IntoResponse> {
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
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let backend = match json.backend {
        CreateConfig::Local { path } => {
            if !path.is_absolute() {
                return Err(error::Error::api(error::ApiErrorKind::NotAbsolutePath));
            }

            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(err) => {
                    match err.kind() {
                        std::io::ErrorKind::NotFound => {
                            return Err(error::Error::api(error::ApiErrorKind::DirNotFound));
                        },
                        _ => {
                            return Err(err.into())
                        }
                    }
                }
            };

            if !metadata.is_dir() {
                return Err(error::Error::api(error::ApiErrorKind::NotDirectory));
            }

            tokio::fs::create_dir_all(&path).await?;

            fs::backend::Config::Local(fs::backend::ConfigLocal { path })
        }
    };

    let transaction = conn.transaction().await?;

    let id = state.ids().wait_storage_id()?;
    let created = chrono::Utc::now();

    if !rfs_lib::fs::storage::name_valid(&json.name) {
        return Err(error::Error::api((
            error::ApiErrorKind::ValidationFailed,
            error::Detail::with_key("name")
        )));
    };

    if fs::Storage::name_check(&transaction, &json.name).await?.is_some() {
        return Err(error::Error::api((
            error::ApiErrorKind::AlreadyExists,
            error::Detail::with_key("name")
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
            return Err(error::Error::api(error::ApiErrorKind::InvalidTags));
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
