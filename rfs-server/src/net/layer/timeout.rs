use std::time::Duration;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;

use tower::{Layer, Service};
use tokio::time::Sleep;
use pin_project::pin_project;
use axum::http::StatusCode;

use crate::net::error;

pub enum TimeoutError<E> {
    Service(E),
    Timeout,
}

impl<E> From<E> for TimeoutError<E> {
    fn from(e: E) -> Self {
        TimeoutError::Service(e)
    }
}

impl<E> From<TimeoutError<E>> for error::Error
where
    E: Into<error::Error>
{
    fn from(err: TimeoutError<E>) -> Self {
        match err {
            TimeoutError::Service(e) => e.into(),
            TimeoutError::Timeout => error::Error::api(error::GeneralKind::Timeout)
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
