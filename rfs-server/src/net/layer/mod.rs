pub mod timeout;
pub mod request_id;

pub mod trace {
    use std::time::Duration;

    use axum::http::{Request, Response, HeaderMap, StatusCode};
    use axum::body::BoxBody;
    use hyper::Body;
    use tracing::Span;
    use tower_http::trace::TraceLayer;
    use tower_http::classify::{SharedClassifier, ServerErrorsFailureClass};

    use super::request_id::RequestId;

    pub fn make_span_with(request: &Request<Body>) -> Span {
        let req_id = RequestId::try_get(request).expect("missing request id");

        tracing::info_span!(
            "REQ",
            i = req_id.id(),
            v = ?request.version(),
            m = %request.method(),
            u = %request.uri(),
            s = tracing::field::Empty
        )
    }

    pub fn on_request(_request: &Request<Body>, _span: &Span) {
        /*
        tracing::info!(
            "{:?} {} {}",
            request.version(),
            request.method(),
            request.uri()
        )
        */
    }

    pub fn on_response(response: &Response<BoxBody>, latency: Duration, span: &Span) {
        span.record("s", &tracing::field::display(response.status()));

        tracing::info!("{:#?}", latency)
    }

    pub fn on_failure(error: ServerErrorsFailureClass, latency: Duration, _span: &Span) {
        tracing::error!("{} {:#?}", error, latency)
    }
}

