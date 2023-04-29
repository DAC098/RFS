use std::path::Path;

use tokio::fs::{File, OpenOptions};
use tokio_util::io::ReaderStream;
use axum::http::StatusCode;
use axum::body::StreamBody;
use axum::response::Response;

use crate::net;
use crate::net::error;

pub async fn stream_file<P>(path: P) -> error::Result<Response<StreamBody<ReaderStream<File>>>>
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
        .body(StreamBody::new(stream))?)
}
