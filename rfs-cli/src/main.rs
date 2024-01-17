use std::path::PathBuf;

use rfs_api::client::ApiClient;
use clap::{ArgMatches};

mod error;
mod input;
mod auth;
mod util;
mod commands;

use error::Context;

fn main() {
    use tracing_subscriber::{FmtSubscriber, EnvFilter};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .expect("failed to initialize global tracing subscriber");

    let end_result = run();

    if let Err(err) = end_result {
        println!("{}", err);
    }
}

fn run() -> error::Result {
    let app_matches = commands::cli().get_matches();

    let session_file = if let Some(arg) = app_matches.get_one::<PathBuf>("cookies") {
        arg.clone()
    } else {
        let mut current_dir = std::env::current_dir()?;
        current_dir.push("rfs_cookies.json");
        current_dir
    };

    let mut client_builder = ApiClient::builder();
    client_builder.cookie_file(session_file.clone());

    let host = app_matches.get_one::<String>("host").unwrap();
    let port = app_matches.get_one::<u16>("port")
        .cloned()
        .unwrap();

    client_builder.secure(app_matches.get_flag("secure"));
    client_builder.port(Some(port));

    if !client_builder.host(host.clone()) {
        return Err(error::Error::from(format!(
            "cannot set host to the value provided. {}", 
            host
        )));
    }

    let mut client = client_builder.build().context("failed to create api client")?;

    match app_matches.subcommand() {
        None => {
            loop {
                let given = input::read_stdin(">")?;
                let trimmed = given.trim();

                let Ok(args_list) = shell_words::split(&trimmed) else {
                    println!("failed to parse command line args");
                    continue;
                };

                let matches = match commands::interactive().try_get_matches_from(args_list) {
                    Ok(m) => m,
                    Err(err) => {
                        println!("{}", err);
                        continue;
                    }
                };

                let result = match matches.subcommand() {
                    Some(("quit", _quit_matches)) => {
                        return Ok(());
                    },
                    Some((cmd, cmd_matches)) => run_subcommand(&mut client, cmd, cmd_matches),
                    _ => unreachable!()
                };

                if let Err(err) = result {
                    println!("{}", err);
                }
            }
        },
        Some((cmd, cmd_matches)) => run_subcommand(&mut client, cmd, cmd_matches)?
    }

    Ok(())
}

fn run_subcommand(client: &mut ApiClient, command: &str, matches: &ArgMatches) -> error::Result {
    match command {
        "connect" => commands::connect(client),
        "disconnect" => commands::disconnect(client),
        "hash" => commands::hash(matches),
        "ping" => commands::ping(client),
        "storage" => commands::storage(client, matches),
        "fs" => commands::fs(client, matches),
        "user" => commands::user(client, matches),
        "auth" => commands::auth(client, matches),
        "sec" => commands::sec(client, matches),
        _ => {
            println!("uknown command");

            Ok(())
        }
    }
}
