use std::path::PathBuf;
use std::str::FromStr;

use rfs_api::client::ApiClient;
use rfs_api::client::auth::session::DropSession;
use rfs_api::client::users::password::UpdatePassword;
use clap::{Parser, Subcommand};

use crate::error::{self, Context};

mod fs;
mod user;
mod sec;
mod totp;
mod connect;

/// a cli for interacting with a RFS.
///
/// provides options for modifying data on a server as well as administration 
/// processes. if no command is provided then it will enter interactive mode.
#[derive(Debug, Parser)]
struct Cli {
    /// host name of server
    ///
    /// will be used in a url so the value must be valid for the hostname part
    /// of a url. examples: example.com | 10.0.0.2 | [fd34::2]
    host: String,

    /// port of server
    ///
    /// if no port is provided it will default to 80 (http) or 443 (https)
    #[arg(short, long)]
    port: Option<u16>,

    /// to use http
    ///
    /// will switch to using an insecure channel when communicating with a server
    #[arg(long)]
    insecure: bool,

    /// file that stores session cookies
    ///
    /// if a file is not specified then it will attempt to load 
    /// "rfs_cookies.json" in the current working directory
    #[arg(long)]
    cookies: Option<PathBuf>,

    #[command(subcommand)]
    command: Cmds,
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

    if let Some((host, port)) = host.rsplit_once(':') {
        let port = u16::from_str(port)
            .context(format!("invalid port number given in domain. given: \"{}\"", port))?;

        client_builder.port(Some(port));

        if !client_builder.host(host.to_owned()) {
            return Err(format!(
                "cannot set host to the value provided. {host}"
            ).into());
        }
    } else {
        if !client_builder.host(host.clone()) {
            return Err(format!(
                "cannot set host to the value provided. {host}"
            ).into());
        }
    }

    client_builder.cookie_file(session_file);
    client_builder.secure(!args.insecure);

    if let Some(port) = args.port {
        client_builder.port(Some(port));
    }

    let mut client = client_builder.build()
        .context("failed to create api client")?;

    match args.command {
        Cmds::Connect => connect(&mut client),
        Cmds::Disconnect => disconnect(&mut client),
        Cmds::Password => password(&mut client),
        Cmds::Totp(given) => totp::handle(&mut client, given),
        Cmds::Fs(given) => fs::handle(&mut client, given),
        Cmds::Users(given) => user::handle(&mut client, given),
        Cmds::Sec(given) => sec::handle(&mut client, given),
        Cmds::Ping => ping(&mut client),
    }
}

#[derive(Debug, Subcommand)]
enum Cmds {
    /// login to the specified server
    #[command(alias = "login")]
    Connect,

    /// logout from the specified serverr
    #[command(alias = "logout")]
    Disconnect,

    /// updates the current password to a new one
    Password,

    /// interacts with data specific to totp 2FA
    Totp(totp::TotpArgs),

    /// interacts with fs items on a server
    Fs(fs::FsArgs),

    /// interacts with users on a server
    Users(user::UsersArgs),

    /// helps to manage security related features on a server
    Sec(sec::SecArgs),

    /// pings the server for activity
    Ping,
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

fn password(client: &mut ApiClient) -> error::Result {
    let current_prompt = "current: ";
    let updated_prompt = "updated: ";
    let confirm_prompt = "confirm: ";

    let current = rpassword::prompt_password(&current_prompt)?;
    let updated = rpassword::prompt_password(&updated_prompt)?;
    let mut confirm;

    loop {
        confirm = rpassword::prompt_password(&confirm_prompt)?;

        if confirm != updated {
            println!("updated and confirm do not match");
        } else {
            break;
        }
    }

    UpdatePassword::update_password(current, updated, confirm)
        .send(client)
        .context("failed to update password")?;

    Ok(())
}

fn ping(client: &mut ApiClient) -> error::Result {
    client.ping().context("failed to ping server")?;

    println!("pong");

    Ok(())
}
