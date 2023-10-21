use clap::{Command, Arg, ArgMatches, value_parser};

use crate::error;
use crate::util;
use crate::state::AppState;

pub fn command() -> Command {
    Command::new("password")
        .subcommand_required(true)
        .about("interacts with password secrets for a server")
        .arg(util::default_help_arg())
        .subcommand(Command::new("get")
            .about("retrieves a list of known password secrets from the server")
            .arg(util::default_help_arg())
            .arg(Arg::new("version")
                .long("version")
                .value_parser(value_parser!(u64))
                .help("the desired secret version to retrieve")
            )
        )
        .subcommand(Command::new("update")
            .about("creats a new password secret that will be used for new passwords")
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("remove")
            .about("removes a desired password secret from the server")
            .after_help("Note: if a password uses the version requested then the server will be unable to hash the password on login")
            .arg(Arg::new("version")
                .long("version")
                .value_parser(value_parser!(u64))
                .help("the desired secret version to remove")
                .required(true)
            )
        )
}

pub fn get(state: &mut AppState, args: &ArgMatches) -> error::Result {
    if let Some(version) = args.get_one::<u64>("version") {
        let path = format!("/sec/secrets/password/{}", version);
        let url = state.server.url.join(&path)?;
        let res = state.client.get(url)
            .send()?;

        let status = res.status();

        if status != reqwest::StatusCode::OK {
            let json = res.json::<rfs_lib::json::Error>()?;

            return Err(error::Error::new()
                .kind("FailedPasswordSecretsLookup")
                .message("failed to retrieve password secrets version")
                .source(json));
        }

        let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::sec::PasswordVersion>>()?;

        println!("{:?}", result);
    } else {
        let path = "/sec/secrets/password";
        let url = state.server.url.join(path)?;
        let res = state.client.get(url)
            .send()?;

        let status = res.status();

        if status != reqwest::StatusCode::OK {
            let json = res.json::<rfs_lib::json::Error>()?;

            return Err(error::Error::new()
                .kind("FailedPasswordSecretsLookup")
                .message("failed to retrieve known password secrets")
                .source(json));
        }

        let result: rfs_lib::json::ListWrapper<Vec<rfs_lib::schema::sec::PasswordListItem>> = res.json()?;

        println!("{:?}", result);
    }

    Ok(())
}

pub fn update(state: &mut AppState, _args: &ArgMatches) -> error::Result {
    let path = "/sec/secrets/password";
    let url = state.server.url.join(path)?;
    let res = state.client.post(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedPasswordSecretsUpdate")
            .message("failed to update password secrets")
            .source(json));
    }

    println!("updated password secrets with new value");

    Ok(())
}

pub fn remove(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let version = args.get_one::<u64>("version").unwrap();
    let path = format!("/sec/secrets/password/{}", version);
    let url = state.server.url.join(&path)?;
    let res = state.client.delete(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedPasswordSecretsRemove")
            .message("failed to remove password secret version")
            .source(json));
    }

    println!("removed password secret version");

    Ok(())
}
