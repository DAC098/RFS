use std::path::PathBuf;

use rfs_api::client::ApiClient;
use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

mod storage;
mod fs;
mod user;
mod auth;
mod sec;

fn append_subcommands(command: Command) -> Command {
    command
        .subcommand(storage::command())
        .subcommand(fs::command())
        .subcommand(user::command())
        .subcommand(auth::command())
        .subcommand(sec::command())
        .subcommand(Command::new("connect")
            .alias("login")
            .about("logs in into the desired server")
        )
        .subcommand(Command::new("disconnect")
            .alias("logout")
            .about("logs out of the desired server")
        )
        .subcommand(Command::new("hash")
            .about("create a hash from a specified file")
            .arg(util::default_help_arg())
            .arg(Arg::new("file")
                .short('f')
                .long("file")
                .value_parser(value_parser!(PathBuf))
                .help("the desired file to hash")
                .required(true)
            )
        )
        .subcommand(Command::new("ping")
            .about("pings server for activity")
        )
}

pub fn cli() -> Command {
    let command = Command::new("rfs-cli")
        .disable_help_flag(true)
        .arg(util::default_help_arg())
        .arg(Arg::new("cookies")
            .long("cookies")
            .value_parser(value_parser!(PathBuf))
            .help("specifies a specific file to save any cookie data to")
        )
        .arg(Arg::new("host")
            .long("host")
            .short('h')
            .default_value("localhost")
            .help("the desired hostname to connect to")
        )
        .arg(Arg::new("port")
            .long("port")
            .short('p')
            .default_value("80")
            .value_parser(value_parser!(u16))
            .help("the desired port to connect to")
        )
        .arg(Arg::new("secure")
            .long("secure")
            .short('s')
            .action(ArgAction::SetTrue)
            .help("sets the connection to use https")
        );

    append_subcommands(command)
}

pub fn interactive() -> Command {
    let command = Command::new("")
        .subcommand_required(true)
        .no_binary_name(true)
        .disable_help_flag(true)
        .arg(util::default_help_arg())
        .subcommand(Command::new("quit")
            .alias("q")
            .about("exits program")
        );

    append_subcommands(command)
}

pub fn connect(client: &mut ApiClient) -> error::Result {
    crate::auth::connect(client)?;

    println!("session authenticated");

    Ok(())
}

pub fn disconnect(client: &mut ApiClient) -> error::Result {
    crate::auth::disconnect(client)?;

    Ok(())
}

pub fn storage(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("create", create_args)) => storage::create(client, create_args),
        Some(("update", update_args)) => storage::update(client, update_args),
        _ => unreachable!()
    }
}

pub fn fs(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("create", create_args)) => fs::create(client, create_args),
        Some(("update", update_args)) => fs::update(client, update_args),
        Some(("upload", upload_args)) => fs::upload(client, upload_args),
        _ => unreachable!()
    }
}

pub fn user(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("group", group_args)) => user::group(client, group_args),
        Some(("create", create_args)) => user::create(client, create_args),
        Some(("update", update_args)) => user::update(client, update_args),
        _ => unreachable!()
    }
}

pub fn auth(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("totp", totp_args)) => auth::totp(client, totp_args),
        _ => unreachable!()
    }
}

pub fn sec(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("secrets", secrets_matches)) => sec::secrets(client, secrets_matches),
        Some(("roles", roles_matches)) => sec::roles(client, roles_matches),
        _ => unreachable!()
    }
}

pub fn hash(args: &ArgMatches) -> error::Result {
    use std::io::{BufReader, BufRead, ErrorKind};
    use std::fs::OpenOptions;

    let mut file_path = args.get_one::<PathBuf>("file").cloned().unwrap();

    if !file_path.is_absolute() {
        let mut cwd = std::env::current_dir()?;
        cwd.push(&file_path);

        file_path = cwd.canonicalize()?;
    }

    let metadata = match file_path.metadata() {
        Ok(m) => m,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                return Err(error::Error::new()
                    .context("requested file was not found"));
            },
            _ => {
                return Err(error::Error::new()
                    .context("failed to read data about the desired file")
                    .source(err));
            }
        }
    };

    if !metadata.is_file() {
        return Err(error::Error::new()
            .context("requested file path is not a file"));
    }

    let mut hasher = blake3::Hasher::new();
    let file = OpenOptions::new()
        .read(true)
        .open(&file_path)?;

    let mut reader = BufReader::with_capacity(1024 * 4, file);

    loop {
        let read = {
            let buffer = reader.fill_buf()?;

            if buffer.len() == 0 {
                break;
            }

            hasher.update(buffer);

            buffer.len()
        };

        reader.consume(read);
    }

    let hash = hasher.finalize();

    println!("{}", hash.to_hex());

    Ok(())
}

pub fn ping(client: &mut ApiClient) -> error::Result {
    client.ping().context("failed to ping server")?;

    println!("pong");

    Ok(())
}
