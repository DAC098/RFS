mod error;
mod input;
mod util;
mod cli;
mod formatting;

fn main() {
    use tracing_subscriber::{FmtSubscriber, EnvFilter};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .expect("failed to initialize global tracing subscriber");

    if let Err(err) = cli::start() {
        println!("{}", err);
    }
}

