use axum::http::{header, HeaderValue, HeaderMap, StatusCode};
use axum::response::Response;

use crate::net::error;

#[inline]
pub fn html_response(contents: String) -> error::Result<Response<String>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html")
        .header("content-length", contents.len())
        .body(contents)?)
}

/// returns an iterator of requested "accept" header values if present
pub fn get_accept_header(hd: &HeaderMap<HeaderValue>) -> error::Result<Option<mime::MimeIter>> {
    if let Some(accept) = hd.get(header::ACCEPT) {
        Ok(Some(mime::MimeIter::new(accept.to_str()?)))
    } else {
        Ok(None)
    }
}

/// checks if header map contains "accept" and is "text/html"
pub fn is_html_accept(hd: &HeaderMap<HeaderValue>) -> error::Result<Option<mime::Mime>> {
    if let Some(mut accept) = get_accept_header(hd)? {
        while let Some(check) = accept.next() {
            if let Ok(part) = check {
                if part.type_() == "text" && part.subtype() == "html" {
                    return Ok(Some(part));
                }
            } else {
                // failed to parse into a mime::Mime struct. do something?
            }
        }
    }

    Ok(None)
}
