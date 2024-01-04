use std::task::{Context, Poll};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tower::{Layer, Service};
use axum::http::Request;

type Counter = Arc<AtomicU64>;

#[derive(Debug, Clone)]
pub struct RequestId {
    id: u64,
}

impl RequestId {
    pub fn try_get<'a, B>(req: &'a Request<B>) -> Option<&'a Self> {
        req.extensions().get()
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


