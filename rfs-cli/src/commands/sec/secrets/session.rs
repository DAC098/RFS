use clap::{Command, ArgMatches};

use crate::error;
use crate::util;
use crate::state::AppState;

pub fn command() -> Command {
    Command::new("session")
        .subcommand_required(true)
        .about("interacts with session secrets for a server")
        .arg(util::default_help_arg())
        .subcommand(Command::new("get")
            .about("retrieves a list of known session secrets from the server")
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("update")
            .about("creates a new session secret that will be used for new session tokens")
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("remove")
            .about("removes the oldest session secret from the server")
            .arg(util::default_help_arg())
        )
}

pub fn get(state: &mut AppState, _args: &ArgMatches) -> error::Result {
    let path = "/sec/secrets/session";
    let url = state.server.url.join(path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedSessionSecretsLookup")
            .message("failed to retrieve known session secrets")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::ListWrapper<rfs_lib::schema::sec::SessionListItem>>()?;

    println!("{:?}", result);

    Ok(())
}

pub fn update(state: &mut AppState, _args: &ArgMatches) -> error::Result {
    let path = "/sec/secrets/session";
    let url = state.server.url.join(path)?;
    let res = state.client.post(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedSessionSecretsUpdate")
            .message("failed to update session secrets")
            .source(json));
    }

    println!("updated session secrets with new value");

    Ok(())
}

pub fn remove(state: &mut AppState, _args: &ArgMatches) -> error::Result {
    let path = "/sec/secrets/session";
    let url = state.server.url.join(path)?;
    let res = state.client.delete(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedSessionSecretsRemove")
            .message("failed to remove session secret")
            .source(json));
    }

    println!("removed session secret");

    Ok(())
}
