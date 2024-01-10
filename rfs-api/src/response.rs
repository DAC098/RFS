use http::StatusCode;
use axum_core::body::Body;
use axum_core::response::Response;
use serde::Serialize;
use bytes::{BytesMut, BufMut};

pub fn serialize_json(
    status: StatusCode,
    data: &impl Serialize
) -> Result<Response, serde_json::Error> {
    let froze = {
        let mut buf = BytesMut::with_capacity(128).writer();
        serde_json::to_writer(&mut buf, data)?;

        buf.into_inner().freeze()
    };

    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("content-length", froze.len())
        .body(Body::from(froze))
        .unwrap())
}

pub fn error_json() -> Response {
    let body = r#"{"kind":"InternalFailure"}"#;

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}
