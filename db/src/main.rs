mod error;
mod args;
mod conn;
mod run;

fn commands() -> clap::Command {
    use clap::{Command, Arg, ArgAction, value_parser};

    Command::new("db")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("setup")
                .about("creates the database from scratch")
                .arg(
                    Arg::new("rollback")
                        .long("rollback")
                        .action(ArgAction::SetTrue)
                        .help("rollback changes made to the database")
                )
                .arg(args::db::connect())
                .arg(args::db::user())
                .arg(args::db::password())
                .arg(args::db::req_password())
                .arg(args::db::host())
                .arg(args::db::port())
                .arg(args::db::dbname())
        )
        .subcommand(
            Command::new("migrate")
                .about("databasae migration operations")
                .arg(args::db::connect())
                .arg(args::db::user())
                .arg(args::db::password())
                .arg(args::db::req_password())
                .arg(args::db::host())
                .arg(args::db::port())
                .arg(args::db::dbname())
                .subcommand(
                    Command::new("run")
                        .about("runs migrates")
                        .arg(
                            Arg::new("groups")
                                .short('g')
                                .long("group")
                                .action(ArgAction::SetTrue)
                                .help("grups migrates into a single transaction")
                        )
                        .arg(
                            Arg::new("abort-divergent")
                                .long("continue-divergent")
                                .action(ArgAction::SetFalse)
                                .help("process will continue if divergent migrations are found")
                        )
                        .arg(
                            Arg::new("abort-missing")
                                .long("continue-missing")
                                .action(ArgAction::SetFalse)
                                .help("process will continue if missing migrates are found")
                        )
                )
                .subcommand(
                    Command::new("list")
                        .about("lists currently available migrates")
                )
                .subcommand(
                    Command::new("last-applied")
                        .about("shows the last applied migration to the database")
                )
                .subcommand(
                    Command::new("applied")
                        .about("shows all currently applied migrations for the database")
                )
        )
}

fn main() {
    use tokio::runtime::Builder;
    use tracing_subscriber::{FmtSubscriber, EnvFilter};

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to initialize global tracing subscriber");

    let matches = commands().get_matches();

    let rt = match Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(1)
        .build() {
        Ok(rt) => rt,
        Err(err) => panic!("failed to start tokio runtime, {:#}", err)
    };

    tracing::event!(
        tracing::Level::INFO,
        "started tokio runtime",
    );

    if let Err(err) = rt.block_on(exec(&matches)) {
        match err.into_parts() {
            (kind , Some(msg), Some(err)) => {
                println!("{}: {}\n{}", kind, msg, err);
            },
            (kind, Some(msg), None) => {
                println!("{}: {}", kind, msg);
            },
            (kind, None, Some(err)) => {
                println!("{}: {}", kind, err);
            },
            (kind, None, None) => {
                println!("{}", kind);
            }
        }
    }
}

async fn exec(matches: &clap::ArgMatches) -> error::Result<()> {
    match matches.subcommand() {
        Some(("setup", setup_matches)) => run::setup(&setup_matches).await?,
        Some(("migrate", migrate_matches)) => {},
        _ => unreachable!()
    };

    Ok(())
}
