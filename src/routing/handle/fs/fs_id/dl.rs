use rfs_lib::ids;
use axum::http::StatusCode;
use axum::extract::{Path, State};
use axum::body::Body;
use axum::response::Response;
use tokio::fs::OpenOptions;
use tokio_util::io::ReaderStream;
use serde::Deserialize;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::initiator;
use crate::sec::authz::permission;
use crate::fs;

#[derive(Deserialize)]
pub struct PathParams {
    fs_id: ids::FSId
}

pub async fn get(
    State(state): State<ArcShared>,
    initiator: initiator::Initiator,
    Path(PathParams { fs_id }): Path<PathParams>,
) -> error::Result<Response<Body>> {
    let conn = state.pool().get().await?;

    if !permission::has_ability(
        &conn,
        &initiator.user.id,
        permission::Scope::Fs,
        permission::Ability::Read
    ).await? {
        return Err(error::Error::api(error::ApiErrorKind::PermissionDenied));
    }

    let item = fs::fetch_item(&conn, &fs_id, &initiator).await?;
    let storage = fs::fetch_storage(&conn, item.storage_id()).await?;

    let Ok(file): Result<fs::File, _> = item.try_into() else {
        return Err(error::Error::api(error::ApiErrorKind::NotFile));
    };

    let builder = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", file.mime.to_string())
        .header("content-length", file.size);

    match fs::backend::Pair::match_up(&storage.backend, &file.backend)? {
        fs::backend::Pair::Local((local, node_local)) => {
            let full = local.path.join(&node_local.path);
            let stream = ReaderStream::new(OpenOptions::new()
                .read(true)
                .open(full)
                .await?);

            Ok(builder.body(Body::from_stream(stream))?)
        }
    }
}
