use std::path::Path;

use tokio::fs::OpenOptions;
use tokio_util::io::ReaderStream;
use axum::http::StatusCode;
use axum::body::Body;
use axum::response::Response;

use crate::net;
use crate::net::error;

pub async fn stream_file<P>(path: P) -> error::Result<Response<Body>>
where
    P: AsRef<Path>,
{
    let path_ref = path.as_ref();
    let mime = net::mime::mime_from_ext(path_ref.extension());

    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .await?;
    let metadata = file.metadata().await?;

    let stream = ReaderStream::new(file);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", mime.to_string())
        .header("content-length", metadata.len())
        .body(Body::from_stream(stream))?)
}

pub async fn response_file<N, P>(name: N, path: P) -> error::Result<Response<Body>>
where
    N: AsRef<str>,
    P: AsRef<Path>,
{
    let path_ref = path.as_ref();

    if !path_ref.try_exists()? {
        Err(error::Error::api((
            error::GeneralKind::NotFound,
            format!("{} was not found", name.as_ref())
        )))
    } else {
        Ok(stream_file(path_ref).await?)
    }
}
