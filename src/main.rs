use std::sync::Arc;

use tracing_subscriber::{FmtSubscriber, EnvFilter};
use futures::StreamExt;
use futures::stream::FuturesUnordered;

mod error;
mod time;
mod sql;
mod net;
mod fs;
mod template;
mod user;
mod sec;
mod state;
mod tags;
//mod storage;
mod routing;
mod config;
mod jobs;

fn main() {
    use tokio::runtime::Builder;

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .expect("failed to initialize global tracing subscriber");

    let rt = match Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(4)
        .build() {
        Ok(rt) => rt,
        Err(err) => {
            panic!("failed to start tokio runtime. {}", err);
        }
    };

    tracing::event!(
        tracing::Level::INFO,
        "started tokio runtime"
    );

    if let Err(err) = rt.block_on(init()) {
        tracing::error!("{err}");
    }
}

async fn init() -> error::Result<()> {
    let config = config::get_config()?;
    let state = Arc::new(state::Shared::from_config(&config)?);
    let mut all_futs = FuturesUnordered::new();

    all_futs.extend(jobs::background(&state, config.settings.data.clone())?);

    let router = routing::routes(&state);

    for (key, listener) in config.settings.listeners {
        let instance_router = router.clone();

        all_futs.push(tokio::spawn(async move {
            let tcp_listener = match std::net::TcpListener::bind(listener.addr) {
                Ok(l) => l,
                Err(err) => {
                    tracing::error!("\"{key}\" failed to bind to socket address: {err}");

                    return;
                }
            };

            match tcp_listener.local_addr() {
                Ok(addr) => {
                    tracing::info!("\"{key}\" tcp socket listener: {addr}");
                }
                Err(err) => {
                    tracing::error!("\"{key}\" failed to retrieve tcp listener address: {err}");
                }
            }

            let fut = axum_server::from_tcp(tcp_listener)
                .serve(instance_router.into_make_service());

            if let Err(err) = fut.await {
                tracing::error!("\"{key}\" server error: {err}");
            }
        }));
    }

    while let Some(_) = all_futs.next().await {
    }

    Ok(())
}

