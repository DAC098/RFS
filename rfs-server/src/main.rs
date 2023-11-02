use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use axum::Router;
use axum::error_handling::HandleErrorLayer;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{FmtSubscriber, EnvFilter};

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
mod storage;
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
        match err.into_parts() {
            (kind, Some(msg), Some(err)) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "{}: {}\n{}",
                    kind,
                    msg,
                    err
                );
            },
            (kind, Some(msg), None) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "{}: {}",
                    kind,
                    msg
                );
            },
            (kind, None, Some(err)) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "{}: {}",
                    kind,
                    err
                );
            },
            (kind, None, None) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "{}",
                    kind
                );
            }
        }
    }
}

async fn init() -> error::Result<()> {
    use axum::routing::{get, post};

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
            "/auth/request",
            post(routing::handle::auth::request::post)
        )
        .route(
            "/auth/submit",
            post(routing::handle::auth::submit::post)
        )
        .route(
            "/auth/verify",
            post(routing::handle::auth::verify::post)
        )
        .route(
            "/auth/password",
            post(routing::handle::auth::password::post)
                .delete(routing::handle::auth::password::delete)
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
            "/storage",
            get(routing::handle::storage::get)
                .post(routing::handle::storage::post)
        )
        .route(
            "/storage/:storage_id",
            get(routing::handle::storage::storage_id::get)
                .put(routing::handle::storage::storage_id::put)
                .delete(routing::handle::storage::storage_id::delete)
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
            get(routing::okay)
        )
        .route(
            "/fs/:fs_id/data",
            get(routing::okay)
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
                //.delete(routing::handle::user::group::group_id::delete)
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

    let sock_addr = config.settings.listen_socket();
    let server = hyper::Server::try_bind(&sock_addr)
        .map_err(|error| error::Error::new()
            .message(format!("failed to bind to socket address: {:#?}", sock_addr))
            .source(error)
        )?
        .serve(router.into_make_service());

    tracing::info!(
        addr = %server.local_addr(),
        "server listening",
    );

    if let Err(err) = server.await {
        Err(error::Error::new()
            .message("server error")
            .source(err))
    } else {
        Ok(())
    }
}

