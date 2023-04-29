use std::path::PathBuf;
use std::str::FromStr;
use std::net::SocketAddr;

use clap::ArgMatches;
use axum::{
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{FmtSubscriber, EnvFilter};

mod error;
mod net;
mod state;
mod routing;

fn commands() -> clap::Command {
    use clap::{Command, Arg, ArgAction, value_parser};

    Command::new("personal-site")
        .arg(Arg::new("ip")
            .short('i')
            .long("ip")
            .action(ArgAction::Set)
            .help("ip address to bind the server to")
        )
        .arg(Arg::new("port")
            .short('p')
            .long("port")
            .action(ArgAction::Set)
            .value_parser(value_parser!(u16))
            .help("port for the server to listen on")
        )
        .arg(Arg::new("assets")
            .long("assets")
            .action(ArgAction::Set)
            .value_parser(value_parser!(PathBuf))
            .help("specifies the directory to load assets from")
        )
        .arg(Arg::new("pages")
            .long("pages")
            .action(ArgAction::Set)
            .value_parser(value_parser!(PathBuf))
            .help("specifies the directory to load html pages from")
        )
        .arg(Arg::new("templates")
            .long("templates")
            .action(ArgAction::Set)
            .value_parser(value_parser!(PathBuf))
            .help("specifies the directory to load handlebars templates from")
        )
        .arg(Arg::new("hbs_dev_mode")
            .long("hbs-dev-mode")
            .action(ArgAction::SetTrue)
            .help("enabled dev mode for handlebars templates")
        )
}

fn main() {
    use tokio::runtime::Builder;

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to initalize global tracing subscriber");

    let matches = commands().get_matches();

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
            (Some(msg), Some(err)) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "Error: {}\n{}",
                    msg,
                    err
                );
            },
            (Some(msg), None) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "Error: {}",
                    msg
                );
            },
            (None, Some(err)) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "Error: {}",
                    err
                );
            },
            (None, None) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    "Error: no additional data"
                );
            }
        }
    }
}

fn get_sock_addr(arg: &ArgMatches) -> error::Result<SocketAddr> {
    use std::net::IpAddr;

    let port = arg.get_one("port")
        .map(|v: &u16| v.clone())
        .unwrap_or(0);

    let ip_addr = if let Some(ip) = arg.get_one::<&str>("ip") {
        IpAddr::from_str(ip).map_err(|_|
            error::Error::new()
                .message("invalid ip address provided")
        )?
    } else {
        IpAddr::from([0,0,0,0])
    };

    Ok(SocketAddr::new(ip_addr, port))
}

fn get_shared_state(arg: &ArgMatches) -> error::Result<state::Shared> {
    let mut state_builder = state::Shared::builder();

    if let Some(path) = arg.get_one::<PathBuf>("assets") {
        state_builder.with_assets(path);
    }

    if let Some(path) = arg.get_one::<PathBuf>("pages") {
        state_builder.with_pages(path.clone());
    }

    if let Some(path) = arg.get_one::<PathBuf>("templates") {
        state_builder.with_templates(path.clone());
    }

    state_builder.set_hbs_dev_mode(arg.get_flag("hbs_dev_mode"));

    Ok(state_builder.build()?)
}

async fn init(arg: ArgMatches) -> error::Result<()> {
    let sock_addr = get_sock_addr(&arg)?;
    let state = get_shared_state(&arg)?;

    let router = Router::new()
        .route("/", get(routing::handle::get))
        .route("/ping", get(routing::handle::ping::get))
        .fallback(routing::serve_file::handle)
        .layer(ServiceBuilder::new()
            .layer(TraceLayer::new_for_http()))
        .with_state(state);

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

