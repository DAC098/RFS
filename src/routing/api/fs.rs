use std::collections::HashSet;
use std::fmt::Write;
use std::io::ErrorKind as StdIoErrorKind;

use rfs_lib::ids;
use rfs_api::fs::{
    DirectoryMin,
    FileMin,
    ItemMin,
    RootMin,
};

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use deadpool_postgres::GenericClient;
use futures::TryStreamExt;
use serde::Deserialize;
use tokio::fs::OpenOptions;
use tokio_util::io::ReaderStream;

use crate::error::{ApiResult, ApiError};
use crate::error::api::{Detail, Context, ApiErrorKind};
use crate::fs::{self, backend};
use crate::routing::query::PaginationQuery;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::sql;
use crate::state::ArcShared;
use crate::tags;
use crate::db;

mod storage;
mod upload;

pub fn routes() -> Router<ArcShared> {
    Router::new()
        .route("/", get(retrieve))
        .route("/storage", get(storage::retrieve)
            .post(storage::create))
        .route("/storage/:storage_uid", get(storage::retrieve_id)
            .patch(storage::update_id)
            .delete(storage::delete_id))
        .route("/:fs_uid", get(retrieve_id)
            .post(create_item)
            .put(upload::upload_file)
            .patch(update_item)
            .delete(delete_item))
        .route("/:fs_uid/contents", get(retrieve_id_contents))
        .route("/:fs_uid/download", get(download_id))
}

#[derive(Deserialize)]
pub struct PathParams {
    fs_uid: ids::FSUid,
}

async fn retrieve(
    db::Conn(conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::FSId>>,
) -> ApiResult<rfs_api::Payload<Vec<ItemMin>>> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![
            initiator.user.id.local(),
            &last_id,
            &fs::consts::ROOT_TYPE,
            &limit
        ];

        conn.query_raw(
            "\
            select fs.uid, \
                   users.uid, \
                   storage.uid, \
                   fs.basename, \
                   fs.created, \
                   fs.updated \
            from fs \
            left join users on \
                fs.user_id = users.id \
            left join storage on \
                fs.storage_id = storage.id \
            where fs.user_id = $1 and \
                  fs.id > (\
                      select fs.id \
                      from fs \
                      where fs.uid = $2\
                  ) and \
                  fs.fs_type = $3 \
            order by fs.id \
            limit $4",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![initiator.user.id.local(), &fs::consts::ROOT_TYPE, &limit, &offset_num];

        conn.query_raw(
            "\
            select fs.uid, \
                   users.uid, \
                   storage.uid, \
                   fs.basename, \
                   fs.created, \
                   fs.updated \
            from fs \
            left join users on \
                fs.user_id = users.id \
            left join storage on \
                fs.storage_id = storage.id \
            where fs.user_id = $1 and \
                  fs.fs_type = $2 \
            order by fs.id \
            limit $3 \
            offset $4",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(limit as usize);

    while let Some(row) = result.try_next().await? {
        let item = ItemMin::Root(RootMin {
            uid: row.get(0),
            user_uid: row.get(1),
            storage_uid: row.get(2),
            basename: row.get(3),
            created: row.get(4),
            updated: row.get(5),
        });

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

pub async fn retrieve_id(
    db::Conn(conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { fs_uid }): Path<PathParams>,
) -> ApiResult<rfs_api::Payload<rfs_api::fs::Item>> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read,
    ).await?;

    tracing::debug!("looking up fs_uid: {fs_uid}");

    Ok(rfs_api::Payload::new(fs::fetch_item_uid(&conn, &fs_uid, &initiator).await?.into()))
}

async fn create_item(
    db::Conn(mut conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { fs_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::CreateDir>,
) -> ApiResult<(StatusCode, rfs_api::Payload<rfs_api::fs::Item>)> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item_uid(&conn, &fs_uid, &initiator),
        fs::fetch_storage_from_fs_uid(&conn, &fs_uid),
    )?;

    let transaction = conn.transaction().await?;
    let uid = ids::FSUid::gen();
    let user = initiator.user.id.clone();
    let storage_id_set = storage.id.clone();
    let created = chrono::Utc::now();
    let basename = json.basename;

    let comment = if let Some(given) = json.comment {
        if !rfs_lib::fs::comment_valid(&given) {
            return Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("comment")
            )));
        }

        Some(given)
    } else {
        None
    };

    if !rfs_lib::fs::basename_valid(&basename) {
        return Err(ApiError::from((
            ApiErrorKind::ValidationFailed,
            Detail::with_key("basename")
        )));
    }

    let Ok((parent, path, container_backend)) = item.try_into_parent_parts() else {
        return Err(ApiError::from(ApiErrorKind::InvalidType));
    };

    if fs::Item::name_check(&transaction, parent.local(), &basename).await?.is_some() {
        return Err(ApiError::from(ApiErrorKind::AlreadyExists));
    }

    let backend = match backend::Pair::match_up(&storage.backend, &container_backend)? {
        backend::Pair::Local((storage_local, container_local)) => {
            let mut full = storage_local.path.join(&container_local.path);
            full.push(&basename);

            tracing::debug!("new directory path: {:?}", full.display());

            tokio::fs::create_dir(&full).await?;

            backend::Node::Local(fs::backend::NodeLocal {
                path: full.strip_prefix(&storage_local.path)
                    .unwrap()
                    .to_owned()
            })
        }
    };

    tracing::debug!("directory backend: {backend:#?}");

    let id: ids::FSId = {
        let pg_backend = sql::ser_to_sql(&backend);

        let row = transaction.query_one(
            "\
            insert into fs(\
                uid, \
                user_id, \
                storage_id, \
                parent, \
                basename, \
                fs_type, \
                fs_path, \
                backend, \
                comment, \
                created\
            ) values \
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
            returning id",
            &[
                &uid,
                user.local(),
                storage_id_set.local(),
                parent.local(),
                &basename,
                &fs::consts::DIR_TYPE,
                &path,
                &pg_backend,
                &comment,
                &created
            ]
        ).await?;

        row.get(0)
    };

    let tags = if let Some(tags) = json.tags {
        if !tags::validate_map(&tags) {
            return Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("tags")
            )));
        }

        tags::create_tags(&transaction, "fs_tags", "fs_id", &id, &tags).await?;

        tags
    } else {
        Default::default()
    };

    transaction.commit().await?;

    let rtn = fs::Item::Directory(fs::Directory {
        id: ids::FSSet::new(id, uid),
        user,
        storage: storage_id_set,
        backend,
        parent,
        basename,
        path,
        tags,
        comment,
        created,
        updated: None,
        deleted: None
    });

    Ok((StatusCode::CREATED, rfs_api::Payload::new(rtn.into())))
}

async fn update_item(
    db::Conn(mut conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { fs_uid }): Path<PathParams>,
    axum::Json(json): axum::Json<rfs_api::fs::UpdateMetadata>,
) -> ApiResult<rfs_api::Payload<rfs_api::fs::Item>> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    if !json.has_work() {
        return Err(ApiError::from(ApiErrorKind::NoWork));
    }

    let mut item = fs::fetch_item_uid(&conn, &fs_uid, &initiator).await?;

    let transaction = conn.transaction().await?;

    {
        let local_id = *item.id().local();
        let updated = chrono::Utc::now();
        let mut update_query = String::from("update fs set updated = $2");
        let mut update_params = sql::ParamsVec::with_capacity(2);
        update_params.push(&local_id);
        update_params.push(&updated);

        if let Some(comment) = &json.comment {
            if comment.len() == 0 {
                write!(&mut update_query, ", comment = null").unwrap();

                item.set_comment(None);
            } else {
                write!(
                    &mut update_query,
                    ", comment = ${}",
                    sql::push_param(&mut update_params, comment)
                ).unwrap();

                item.set_comment(Some(comment.clone()));
            }
        }

        write!(&mut update_query, " where id = $1").unwrap();

        transaction.execute(update_query.as_str(), update_params.as_slice()).await?;
    }

    if let Some(tags) = json.tags {
        if !tags::validate_map(&tags) {
            return Err(ApiError::from(ApiErrorKind::InvalidTags));
        }

        tags::update_tags(
            &transaction,
            "fs_tags",
            "fs_id",
            item.id().local(),
            &tags
        ).await?;

        item.set_tags(tags);
    }

    transaction.commit().await?;

    Ok(rfs_api::Payload::new(item.into()))
}

async fn delete_item(
    db::Conn(mut conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { fs_uid }): Path<PathParams>,
) -> ApiResult<StatusCode> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Write,
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item_uid(&conn, &fs_uid, &initiator),
        fs::fetch_storage_from_fs_uid(&conn, &fs_uid),
    )?;

    match item {
        fs::Item::Root(_root) => {
            return Err(ApiError::from(ApiErrorKind::NotPermitted));
        },
        fs::Item::Directory(dir) => {
            delete_dir(&mut conn, storage, dir).await?;
        },
        fs::Item::File(file) => {
            delete_file(&mut conn, storage, file).await?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_file(
    conn: &mut impl GenericClient,
    storage: fs::Storage,
    file: fs::File,
) -> ApiResult<()> {
    let transaction = conn.transaction().await?;

    transaction.execute(
        "delete from fs where id = $1",
        &[file.id.local()]
    ).await?;

    match backend::Pair::match_up(&storage.backend, &file.backend)? {
        backend::Pair::Local((local, node_local)) => {
            let full_path = local.path.join(&node_local.path);

            tokio::fs::remove_file(&full_path).await?;
        }
    }

    transaction.commit().await?;

    Ok(())
}

async fn delete_dir(
    conn: &mut impl GenericClient,
    storage: fs::Storage,
    directory: fs::Directory,
) -> ApiResult<()> {
    let transaction = conn.transaction().await?;

    let results = transaction.query_raw(
        "\
        with recursive dir_tree as (\
            select fs_root.id, \
                   fs_root.parent, \
                   fs_root.fs_type, \
                   fs_root.backend, \
                   1 as level, \
                   fs_root.hash \
            from fs fs_root \
            where id = $1 \
            union \
            select fs_contents.id, \
                   fs_contents.parent, \
                   fs_contents.fs_type, \
                   fs_contents.backend, \
                   dir_tree.level + 1 as level, \
                   fs_contents.hash \
            from fs fs_contents \
            inner join dir_tree on dir_tree.id = fs_contents.parent\
        ) \
        select * \
        from dir_tree \
        order by level desc, \
                 parent, \
                 fs_type, \
                 id",
        &[directory.id.local()]
    ).await?;

    futures::pin_mut!(results);

    let mut skip_parents: HashSet<ids::FSId> = HashSet::new();
    let mut deleted: Vec<ids::FSId> = Vec::new();
    let mut failed: Vec<ids::FSId> = Vec::new();
    let mut skipped: Vec<ids::FSId> = Vec::new();

    while let Some(row) = results.try_next().await? {
        let id: ids::FSId = row.get(0);
        let parent: ids::FSId = row.get(1);
        let fs_type: fs::consts::FsType = row.get(2);
        let backend: fs::backend::Node = sql::de_from_sql(row.get(3));
        let level: i32 = row.get(4);

        if skip_parents.contains(&id) {
            tracing::debug!("skipping fs item. id: {id}");

            skipped.push(id);
            skip_parents.insert(parent);

            continue;
        }

        let Ok(pair) = backend::Pair::match_up(&storage.backend, &backend) else {
            tracing::error!("failed to delete item. backend miss-match. id: {id}");

            failed.push(id);
            skip_parents.insert(parent);

            continue;
        };

        match pair {
            backend::Pair::Local((local, node_local)) => {
                let full_path = local.path.join(&node_local.path);

                tracing::debug!("deleting id: {id}\ndepth: {level}\npath: {}", full_path.display());

                match fs_type {
                    fs::consts::FILE_TYPE => {
                        if let Err(err) = tokio::fs::remove_file(&full_path).await {
                            match err.kind() {
                                StdIoErrorKind::NotFound => {
                                    deleted.push(id);
                                }
                                _ => {
                                    tracing::error!("failed to delete file. id: {id} path: {} {err}", full_path.display());

                                    failed.push(id);
                                    skip_parents.insert(parent);
                                }
                            }
                        } else {
                            deleted.push(id);
                        }
                    }
                    fs::consts::DIR_TYPE => {
                        if let Err(err) = tokio::fs::remove_dir(&full_path).await {
                            match err.kind() {
                                StdIoErrorKind::NotFound => {
                                    deleted.push(id);
                                }
                                _ => {
                                    tracing::error!("failed to delete directory. id: {id} path: {} {err}", full_path.display());

                                    failed.push(id);
                                    skip_parents.insert(parent);
                                }
                            }
                        } else {
                            deleted.push(id);
                        }
                    }
                    _ => {
                        tracing::debug!("unhandled file type. id: {id} type: {fs_type}");

                        skipped.push(id);
                        skip_parents.insert(parent);
                    }
                }
            }
        }
    }

    let del_result = transaction.execute(
        "delete from fs where id = any($1)",
        &[&deleted]
    ).await?;

    tracing::debug!("deleted: {del_result} skipped: {} failed: {}", skipped.len(), failed.len());

    transaction.commit().await?;

    Ok(())
}

async fn retrieve_id_contents(
    db::Conn(conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { fs_uid }): Path<PathParams>,
    Query(PaginationQuery { limit, offset, last_id }): Query<PaginationQuery<ids::FSUid>>,
) -> ApiResult<rfs_api::Payload<Vec<ItemMin>>> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read,
    ).await?;

    let item = fs::fetch_item_uid(&conn, &fs_uid, &initiator).await?;

    let container = item.as_container()
        .kind(ApiErrorKind::NotDirectory)?;

    let mut pagination = rfs_api::Pagination::from(&limit);

    let result = if let Some(last_id) = last_id {
        let params: sql::ParamsVec = vec![container.id(), &last_id, &limit];

        conn.query_raw(
            "\
            select fs.uid, \
                   users.uid, \
                   storage.uid, \
                   fs_parent.uid, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.created, \
                   fs.updated \
            from fs \
            left join users on \
                fs.user_id = users.id \
            left join storage on \
                fs.storage_id = storage.id \
            left join fs as fs_parent on \
                fs.parent = fs_parent.id \
            where fs.parent = $1 and fs.id > (\
                select fs.id \
                from fs \
                where fs.uid = $2\
            ) \
            order by fs.id \
            limit $3",
            params
        ).await?
    } else {
        pagination.set_offset(offset);

        let offset_num = limit.sql_offset(offset);
        let params: sql::ParamsVec = vec![container.id(), &limit, &offset_num];

        conn.query_raw(
            "\
            select fs.uid, \
                   users.uid, \
                   storage.uid, \
                   fs_parent.uid, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.created, \
                   fs.updated \
            from fs \
            left join users on \
                fs.user_id = users.id \
            left join storage on \
                fs.storage_id = storage.id \
            left join fs as fs_parent on \
                fs.parent = fs_parent.id \
            where fs.parent = $1 \
            order by fs.id \
            limit $2 \
            offset $3",
            params
        ).await?
    };

    futures::pin_mut!(result);

    let mut list = Vec::with_capacity(limit as usize);

    while let Some(row) = result.try_next().await? {
        let fs_type = row.get(5);

        let item = match fs_type {
            fs::consts::ROOT_TYPE => {
                ItemMin::Root(RootMin {
                    uid: row.get(0),
                    user_uid: row.get(1),
                    storage_uid: row.get(2),
                    basename: row.get(4),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            fs::consts::FILE_TYPE => {
                ItemMin::File(FileMin {
                    uid: row.get(0),
                    user_uid: row.get(1),
                    storage_uid: row.get(2),
                    parent: row.get(3),
                    basename: row.get(4),
                    path: row.get(6),
                    size: sql::u64_from_sql(row.get(7)),
                    mime: sql::mime_from_sql(row.get(8), row.get(9)),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            fs::consts::DIR_TYPE => {
                ItemMin::Directory(DirectoryMin {
                    uid: row.get(0),
                    user_uid: row.get(1),
                    storage_uid: row.get(2),
                    parent: row.get(3),
                    basename: row.get(4),
                    path: row.get(6),
                    created: row.get(10),
                    updated: row.get(11),
                })
            }
            _ => {
                panic!("unexpected fs_type when retrieving fs item. type: {fs_type}");
            }
        };

        list.push(item);
    }

    Ok(rfs_api::Payload::from((pagination, list)))
}

async fn download_id(
    db::Conn(conn): db::Conn,
    rbac: permission::Rbac,
    initiator: initiator::Initiator,
    Path(PathParams { fs_uid }): Path<PathParams>,
) -> ApiResult<Response<Body>> {
    rbac.api_ability(
        &conn,
        &initiator,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await?;

    let (item, storage) = tokio::try_join!(
        fs::fetch_item_uid(&conn, &fs_uid, &initiator),
        fs::fetch_storage_from_fs_uid(&conn, &fs_uid),
    )?;

    let Ok(file): Result<fs::File, _> = item.try_into() else {
        return Err(ApiError::from(ApiErrorKind::NotFile));
    };

    let builder = Response::builder()
        .status(StatusCode::OK)
        .header("content-disposition", format!("attachment; filename=\"{}\"", file.basename))
        .header("content-type", file.mime.to_string())
        .header("content-length", file.size)
        .header("x-checksum", format!("blake3:{}", file.hash));

    match backend::Pair::match_up(&storage.backend, &file.backend)? {
        backend::Pair::Local((local, node_local)) => {
            let full = local.path.join(&node_local.path);
            let stream = ReaderStream::new(OpenOptions::new()
                .read(true)
                .open(full)
                .await?);

            Ok(builder.body(Body::from_stream(stream))?)
        }
    }
}
