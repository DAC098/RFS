use std::path::PathBuf;

use rfs_api::client::ApiClient;
use rfs_api::client::auth::session::DropSession;
use clap::{Parser, Subcommand, Args};

use crate::error::{self, Context};
use crate::input;

mod storage;
mod fs;
mod user;
mod auth;
mod sec;

mod connect;

/// a cli for interacting with a RFS.
///
/// provides options for modifying data on a server as well as administration 
/// processes. if no command is provided then it will enter interactive mode.
#[derive(Debug, Parser)]
struct Cli {
    /// file that stores session cookies
    ///
    /// if a file is not specified then it will attempt to load 
    /// "rfs_cookies.json" in the current working directory
    #[arg(long)]
    cookies: Option<PathBuf>,

    /// host name of server
    ///
    /// will be used in a url so the value must be valid for the hostname part
    /// of a url. examples: example.com | 10.0.0.2 | [fd34::2]
    #[arg(short = 'H', long)]
    host: Option<String>,

    /// port of server
    ///
    /// if no port is provided it will default to 80 (http) or 443 (https)
    #[arg(short, long)]
    port: Option<u16>,

    /// to use https
    ///
    /// will switch to using a secure channel when communicating with a server
    #[arg(short, long)]
    secure: bool,

    #[command(subcommand)]
    command: Option<BaseCmds>
}

pub fn start() -> error::Result {
    let args = Cli::parse();

    let session_file = if let Some(arg) = args.cookies {
        arg.clone()
    } else {
        let mut current_dir = std::env::current_dir()?;
        current_dir.push("rfs_cookies.json");
        current_dir
    };

    let mut client_builder = ApiClient::builder();
    client_builder.cookie_file(session_file);

    client_builder.secure(args.secure);
    client_builder.port(args.port);

    if let Some(host) = args.host {
        if !client_builder.host(host.clone()) {
            return Err(error::Error::from(format!(
                "cannot set host to the value provided. {}",
                host
            )));
        }
    }

    let mut client = client_builder.build().context("failed to create api client")?;

    match args.command {
        Some(cmd) => handle(&mut client, cmd),
        None => Interactive::handle(&mut client)
    }
}

#[derive(Debug, Parser)]
enum Interactive {
    #[command(flatten)]
    Base(BaseCmds),
    Quit
}

impl Interactive {
    fn handle(client: &mut ApiClient) -> error::Result {
        loop {
            let given = input::read_stdin(">")?;
            let trimmed = given.trim();

            let Ok(args_list) = shell_words::split(&trimmed) else {
                println!("failed to parse command line args");
                continue;
            };

            let cmd = match Interactive::try_parse_from(args_list) {
                Ok(c) => c,
                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            };

            let result = match cmd {
                Interactive::Base(cmd) => handle(client, cmd),
                Interactive::Quit => break,
            };

            if let Err(err) = result {
                println!("{}", err);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum BaseCmds {
    /// login to the specified server
    #[command(alias = "login")]
    Connect,

    /// logout from the specified serverr
    #[command(alias = "logout")]
    Disconnect,

    /// create a hash from a specified file
    Hash(HashArgs),

    /// pings the server for activity
    Ping,

    /// interacts with storage mediums on a server
    Storage(storage::StorageArgs),

    /// interacts with fs items on a server
    Fs(fs::FsArgs),

    /// interacts with users on a server
    Users(user::UsersArgs),

    /// interacts with auth data for the current user
    Auth(auth::AuthArgs),

    /// helps to manage security related features on a server
    Sec(sec::SecArgs),

}

fn handle(client: &mut ApiClient, command: BaseCmds) -> error::Result {
    match command {
        BaseCmds::Connect => connect(client),
        BaseCmds::Disconnect => disconnect(client),
        BaseCmds::Storage(given) => storage::handle(client, given),
        BaseCmds::Fs(given) => fs::handle(client, given),
        BaseCmds::Users(given) => user::handle(client, given),
        BaseCmds::Auth(given) => auth::handle(client, given),
        BaseCmds::Sec(given) => sec::handle(client, given),
        BaseCmds::Hash(given) => hash(given),
        BaseCmds::Ping => ping(client),
    }
}

fn connect(client: &mut ApiClient) -> error::Result {
    let Some(auth_method) = connect::submit_user(client)? else {
        return Ok(());
    };

    let Some(verify_method) = connect::submit_auth(client, auth_method)? else {
        return Ok(());
    };

    connect::submit_verify(client, verify_method)
}

fn disconnect(client: &mut ApiClient) -> error::Result {
    DropSession::new().send(client)?;

    client.save_session().context("failed saving session data")?;

    Ok(())
}

#[derive(Debug, Args)]
struct HashArgs {
    /// the desired file to hash
    #[arg(short, long)]
    file: PathBuf,
}

fn hash(mut args: HashArgs) -> error::Result {
    use std::io::{BufReader, BufRead, ErrorKind};
    use std::fs::OpenOptions;

    if !args.file.is_absolute() {
        let mut cwd = std::env::current_dir()?;
        cwd.push(&args.file);

        args.file = cwd.canonicalize()?;
    }

    let metadata = match args.file.metadata() {
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
        .open(&args.file)?;

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

fn ping(client: &mut ApiClient) -> error::Result {
    client.ping().context("failed to ping server")?;

    println!("pong");

    Ok(())
}
