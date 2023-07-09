use std::path::PathBuf;
use std::str::FromStr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use axum::Router;
use axum::error_handling::HandleErrorLayer;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{FmtSubscriber, EnvFilter};

mod error;
mod util;
mod net;
mod fs;
mod template;
mod user;
mod sec;
mod state;
mod tags;
mod storage;
mod routing;

#[derive(clap::Parser, Debug)]
#[command(author, version, version, about, long_about = None)]
struct CommandArgs {
    /// ip address to bind the server to
    #[arg(short, long)]
    ip: Option<String>,

    /// port for the server to listen on
    #[arg(short, long)]
    port: Option<u16>,

    /// specified the directory to load assets from
    #[arg(long)]
    assets: Option<PathBuf>,

    /// specifies the directory to load html pages from
    #[arg(long)]
    pages: Option<PathBuf>,

    /// specified the directory to load handlebars templates from
    #[arg(long)]
    templates: Option<PathBuf>,

    /// enabled dev mode for handlebars templates
    #[arg(long)]
    hbs_dev_mode: bool,

    /// postgres username for connecting to database
    #[arg(long)]
    pg_user: Option<String>,

    /// postgres user password for connecting to database
    #[arg(long)]
    pg_password: Option<String>,

    /// postgres host address for database
    #[arg(long)]
    pg_host: Option<String>,

    /// postgres port for connecting to host
    #[arg(long)]
    pg_port: Option<u16>,

    /// postgres database name
    #[arg(long)]
    pg_dbname: Option<String>,

    /// hashing algorithm to use for session key
    #[arg(long)]
    session_hash: Option<sec::state::SessionHash>,

    /// session secret for hashing session ids
    #[arg(long)]
    session_secret: Option<String>
}

fn main() {
    use tokio::runtime::Builder;

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .expect("failed to initialize global tracing subscriber");

    let matches = CommandArgs::parse();

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

    if let Err(err) = rt.block_on(init(matches)) {
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

fn get_sock_addr(arg: &CommandArgs) -> error::Result<SocketAddr> {
    use std::net::IpAddr;

    let ip_addr = if let Some(ip) = &arg.ip {
        IpAddr::from_str(&ip).map_err(|_|
            error::Error::new()
                .message("invalid ip address provided")
        )?
    } else {
        IpAddr::from([0,0,0,0])
    };

    Ok(SocketAddr::new(ip_addr, arg.port.unwrap_or(0)))
}

fn get_shared_state(arg: &CommandArgs) -> error::Result<state::Shared> {
    let mut state_builder = state::Shared::builder();

    if let Some(path) = &arg.assets {
        state_builder.set_assets(path);
    }

    if let Some(path) = &arg.pages {
        state_builder.set_pages(path.clone());
    }

    {
        let templates = state_builder.templates();

        if let Some(path) = &arg.templates {
            templates.set_templates(path.clone());
        }

        templates.set_dev_mode(arg.hbs_dev_mode);
    }

    {
        let pg_options = state_builder.pg_options();

        if let Some(user) = &arg.pg_user {
            pg_options.set_user(user);
        }

        if let Some(password) = &arg.pg_password {
            pg_options.set_password(password);
        }

        if let Some(host) = &arg.pg_host {
            pg_options.set_host(host);
        }

        if let Some(port) = &arg.pg_port {
            pg_options.set_port(*port);
        }

        if let Some(dbname) = &arg.pg_dbname {
            pg_options.set_dbname(dbname);
        }
    }

    {
        let sec = state_builder.sec();

        if let Some(session_secret) = &arg.session_secret {
            sec.set_session_secret(session_secret.clone());
        }

        if let Some(session_hash) = &arg.session_hash { 
            sec.set_session_hash(session_hash.clone());
        }
    }

    tracing::event!(
        tracing::Level::DEBUG,
        "shared state builder {:#?}",
        state_builder
    );

    Ok(state_builder.build()?)
}

async fn init(arg: CommandArgs) -> error::Result<()> {
    use axum::routing::{get, post, put, patch, delete};
    use axum::error_handling::HandleError;

    let sock_addr = get_sock_addr(&arg)?;
    let state = get_shared_state(&arg)?;

    tracing::event!(
        tracing::Level::DEBUG,
        "shared state {:#?}",
        state
    );

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
            get(routing::okay)
                .post(routing::handle::auth::totp::post)
                .delete(routing::handle::auth::totp::delete)
        )
        .route(
            "/auth/totp_hash",
            get(routing::okay)
                .post(routing::handle::auth::totp_hash::post)
        )
        .route(
            "/auth/totp_hash/:key_id",
            put(routing::handle::auth::totp_hash::key_id::put)
                .delete(routing::handle::auth::totp_hash::key_id::delete)
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
            "/storage/:storage_id/fs",
            get(routing::handle::storage::storage_id::fs::get)
                .post(routing::handle::storage::storage_id::fs::post)
        )
        .route(
            "/storage/:storage_id/fs/:fs_id",
            get(routing::handle::storage::storage_id::fs::fs_id::get)
                .post(routing::handle::storage::storage_id::fs::fs_id::post)
                .put(routing::handle::storage::storage_id::fs::fs_id::put)
                .delete(routing::handle::storage::storage_id::fs::fs_id::delete)
        )
        .route(
            "/storage/:storage_id/fs/:fs_id/download",
            get(routing::okay)
        )
        .route(
            "/stroage/:stroage_id/fs/:fs_id/contents",
            get(routing::okay)
        )
        .route(
            "/storage/:storage_id/fs/:fs_id/checksum",
            get(routing::okay)
                .post(routing::okay)
        )
        .route(
            "/storage/:storage_id/fs/:fs_id/checksum/:type",
            get(routing::okay)
                .delete(routing::okay)
        )
        .route(
            "/users",
            get(routing::okay)
                .post(routing::okay)
        )
        .route(
            "/users/:user_id",
            get(routing::okay)
                .put(routing::okay)
                .delete(routing::okay)
        )
        .route(
            "/users/:user_id/bot",
            get(routing::okay)
                .post(routing::okay)
        )
        .route(
            "/users/:user_id/bot/:bot_id",
            get(routing::okay)
                .put(routing::okay)
                .delete(routing::okay)
        )
        .route("/ping", get(routing::handle::ping::get))
        .fallback(routing::serve_file::handle)
        .layer(ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(HandleErrorLayer::new(net::error::handle_error))
            .layer(net::layer::timeout::TimeoutLayer::new(Duration::new(90, 0)))
        )
        .with_state(Arc::new(state));

    let server = hyper::Server::try_bind(&sock_addr)
        .map_err(|error| error::Error::new()
            .message(format!("failed to bind to socket address: {:#?}", sock_addr))
            .source(error)
        )?
        .serve(router.into_make_service());

    tracing::event!(
        tracing::Level::INFO,
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

