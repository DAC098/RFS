use std::time::Duration;
use std::task::{Context, Poll};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::pin::Pin;
use std::future::Future;

use axum::http::{Request, Response, Extensions};
use axum::body::Body;
use pin_project::pin_project;
use tokio::time::Sleep;
use tower::{Layer, Service};
use tower_http::classify::ServerErrorsFailureClass;
use tracing::Span;

use crate::error;

pub fn make_span_with(request: &Request<Body>) -> Span {
    let req_id = RequestId::from_request(request).expect("missing request id");

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

pub fn on_response(response: &Response<Body>, latency: Duration, span: &Span) {
    span.record("s", &tracing::field::display(response.status()));

    tracing::info!("{:#?}", latency)
}

pub fn on_failure(error: ServerErrorsFailureClass, latency: Duration, _span: &Span) {
    tracing::error!("{} {:#?}", error, latency)
}

type Counter = Arc<AtomicU64>;

#[derive(Debug, Clone)]
pub struct RequestId {
    id: u64,
}

impl RequestId {
    pub fn from_request<'a, B>(req: &'a Request<B>) -> Option<&'a Self> {
        Self::from_extensions(req.extensions())
    }

    pub fn from_extensions<'a>(extensions: &'a Extensions) -> Option<&'a Self> {
        extensions.get()
    }

    pub fn id(&self) -> &u64 {
        &self.id
    }
}

#[derive(Debug, Clone)]
pub struct RIDService<S> {
    inner: S,
    counter: Counter
}

impl<S> RIDService<S> {
    pub fn new(inner: S, counter: Counter) -> Self {
        RIDService {
            inner,
            counter
        }
    }
}

impl<S, B> Service<Request<B>> for RIDService<S>
where
    S: Service<Request<B>>
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);

        {
            let extensions = request.extensions_mut();
            extensions.insert(RequestId { id });
        }

        self.inner.call(request)
    }
}

#[derive(Debug, Clone)]
pub struct RIDLayer {
    counter: Counter
}

impl RIDLayer {
    pub fn new() -> Self {
        RIDLayer {
            counter: Arc::new(AtomicU64::new(1))
        }
    }
}

impl<S> Layer<S> for RIDLayer {
    type Service = RIDService<S>;

    fn layer(&self, service: S) -> Self::Service {
        RIDService::new(service, self.counter.clone())
    }
}

pub enum TimeoutError<E> {
    Service(E),
    Timeout,
}

impl<E> From<E> for TimeoutError<E> {
    fn from(e: E) -> Self {
        TimeoutError::Service(e)
    }
}

impl<E> From<TimeoutError<E>> for error::ApiError
where
    E: Into<error::ApiError>
{
    fn from(err: TimeoutError<E>) -> Self {
        match err {
            TimeoutError::Service(e) => e.into(),
            TimeoutError::Timeout => error::ApiError::from(error::api::ApiErrorKind::Timeout)
        }
    }
}

#[pin_project]
pub struct TimeoutFuture<F> {
    #[pin]
    resposne: F,
    #[pin]
    sleep: Sleep,
}

impl<F, Response, Error> Future for TimeoutFuture<F>
where
    F: Future<Output = Result<Response, Error>>,
{
    type Output = Result<Response, TimeoutError<Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        match this.resposne.poll(cx) {
            Poll::Ready(result) => {
                let result = result.map_err(Into::into);

                return Poll::Ready(result);
            },
            Poll::Pending => {}
        }

        match this.sleep.poll(cx) {
            Poll::Ready(()) => Poll::Ready(Err(TimeoutError::Timeout)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Timeout<S> {
    inner: S,
    timeout: Duration,
}

impl<S> Timeout<S> {
    pub fn new(inner: S, timeout: Duration) -> Self {
        Timeout { inner, timeout }
    }
}

impl<S, Request> Service<Request> for Timeout<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = TimeoutError<S::Error>;
    type Future = TimeoutFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let resposne = self.inner.call(request);
        let sleep = tokio::time::sleep(self.timeout);

        TimeoutFuture { resposne, sleep }
    }
}

#[derive(Debug, Clone)]
pub struct TimeoutLayer {
    timeout: Duration,
}

impl TimeoutLayer {
    pub fn new(timeout: Duration) -> Self {
        TimeoutLayer { timeout }
    }
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = Timeout<S>;

    fn layer(&self, service: S) -> Self::Service {
        Timeout::new(service, self.timeout)
    }
}
