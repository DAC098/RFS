use clap::Parser;
use tracing_subscriber::{FmtSubscriber, EnvFilter};

mod error;
mod input;

fn main() {
    use tokio::runtime::Builder;

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to initialize global tracing subscriber");

    let rt = match Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(1)
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

pub async fn init() -> error::Result<()> {
    let given = input::read_stdin(">")?;
    let trimmed = given.trim();

    println!("given \"{}\"", trimmed);

    Ok(())
}
