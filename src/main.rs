use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::error_handling::HandleErrorLayer;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
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
    use axum::routing::{get, post, delete};

    let config = config::get_config()?;
    let state = state::Shared::from_config(&config)?;

    let router = Router::new()
        .route(
            "/",
            get(routing::handle::get)
        )
        .route(
            "/auth",
            get(routing::handle::auth::get)
        )
        .route(
            "/auth/session/request",
            post(routing::handle::auth::session::request::post)
        )
        .route(
            "/auth/session/submit",
            post(routing::handle::auth::session::submit::post)
        )
        .route(
            "/auth/session/verify",
            post(routing::handle::auth::session::verify::post)
        )
        .route(
            "/auth/session/drop",
            delete(routing::handle::auth::session::drop::delete)
        )
        .route(
            "/auth/password",
            post(routing::handle::auth::password::post)
        )
        .route(
            "/auth/totp",
            get(routing::handle::auth::totp::get)
                .post(routing::handle::auth::totp::post)
                .patch(routing::handle::auth::totp::patch)
                .delete(routing::handle::auth::totp::delete)
        )
        .route(
            "/auth/totp/recovery",
            get(routing::handle::auth::totp::recovery::get)
                .post(routing::handle::auth::totp::recovery::post)
        )
        .route(
            "/auth/totp/recovery/:key_id",
            get(routing::handle::auth::totp::recovery::key_id::get)
                .patch(routing::handle::auth::totp::recovery::key_id::patch)
                .delete(routing::handle::auth::totp::recovery::key_id::delete)
        )
        .route(
            "/sec/secrets/password",
            get(routing::handle::sec::secrets::password::get)
                .post(routing::handle::sec::secrets::password::post)
        )
        .route(
            "/sec/secrets/password/:version",
            get(routing::handle::sec::secrets::password::version::get)
                .delete(routing::handle::sec::secrets::password::version::delete)
        )
        .route(
            "/sec/secrets/session",
            get(routing::handle::sec::secrets::session::get)
                .post(routing::handle::sec::secrets::session::post)
                .delete(routing::handle::sec::secrets::session::delete)
        )
        .route(
            "/sec/roles",
            get(routing::handle::sec::roles::get)
                .post(routing::handle::sec::roles::post)
        )
        .route(
            "/sec/roles/:role_id",
            get(routing::handle::sec::roles::role_id::get)
                .patch(routing::handle::sec::roles::role_id::patch)
                .delete(routing::handle::sec::roles::role_id::delete)
        )
        .route(
            "/sec/roles/:role_id/users",
            get(routing::handle::sec::roles::role_id::users::get)
                .post(routing::handle::sec::roles::role_id::users::post)
                .delete(routing::handle::sec::roles::role_id::users::delete)
        )
        .route(
            "/sec/roles/:role_id/groups",
            get(routing::handle::sec::roles::role_id::groups::get)
                .post(routing::handle::sec::roles::role_id::groups::post)
                .delete(routing::handle::sec::roles::role_id::groups::delete)
        )
        .route(
            "/fs/storage",
            get(routing::handle::fs::storage::get)
                .post(routing::handle::fs::storage::post)
        )
        .route(
            "/fs/storage/:storage_id",
            get(routing::handle::fs::storage::storage_id::get)
                .put(routing::handle::fs::storage::storage_id::put)
                .delete(routing::handle::fs::storage::storage_id::delete)
        )
        .route(
            "/fs/roots",
            get(routing::handle::fs::roots::get)
        )
        .route(
            "/fs/:fs_id",
            get(routing::handle::fs::fs_id::get)
                .post(routing::handle::fs::fs_id::post)
                .put(routing::handle::fs::fs_id::put)
                .patch(routing::handle::fs::fs_id::patch)
                .delete(routing::handle::fs::fs_id::delete)
        )
        .route(
            "/fs/:fs_id/contents",
            get(routing::handle::fs::fs_id::contents::get)
        )
        .route(
            "/fs/:fs_id/dl",
            get(routing::handle::fs::fs_id::dl::get)
        )
        .route(
            "/user",
            get(routing::handle::user::get)
                .post(routing::handle::user::post)
        )
        .route(
            "/user/group",
            get(routing::handle::user::group::get)
                .post(routing::handle::user::group::post)
        )
        .route(
            "/user/group/:group_id",
            get(routing::handle::user::group::group_id::get)
                .patch(routing::handle::user::group::group_id::patch)
                .delete(routing::handle::user::group::group_id::delete)
        )
        .route(
            "/user/group/:group_id/users",
            get(routing::handle::user::group::group_id::users::get)
                .post(routing::handle::user::group::group_id::users::post)
                .delete(routing::handle::user::group::group_id::users::delete)
        )
        .route(
            "/user/:user_id",
            get(routing::handle::user::user_id::get)
                .patch(routing::handle::user::user_id::patch)
                .delete(routing::handle::user::user_id::delete)
        )
        .route(
            "/user/:user_id/bot",
            get(routing::okay)
                .post(routing::okay)
        )
        .route(
            "/user/:user_id/bot/:bot_id",
            get(routing::okay)
                .put(routing::okay)
                .delete(routing::okay)
        )
        .route("/ping", get(routing::handle::ping::get))
        .fallback(routing::serve_file::handle)
        .layer(ServiceBuilder::new()
            .layer(net::layer::request_id::RIDLayer::new())
            .layer(TraceLayer::new_for_http()
                .make_span_with(net::layer::trace::make_span_with)
                .on_request(net::layer::trace::on_request)
                .on_response(net::layer::trace::on_response)
                .on_failure(net::layer::trace::on_failure))
            .layer(HandleErrorLayer::new(net::error::handle_error))
            .layer(net::layer::timeout::TimeoutLayer::new(Duration::new(90, 0)))
        )
        .with_state(Arc::new(state));

    let mut all_futs = FuturesUnordered::new();

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

