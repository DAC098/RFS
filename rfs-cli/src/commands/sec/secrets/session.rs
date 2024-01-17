use rfs_api::client::ApiClient;
use rfs_api::client::sec::secrets::{
    CreateSessionSecret,
    DeleteSessionSecret,
    QuerySessionSecrets,
};

use clap::{Command, ArgMatches};

use crate::error::{self, Context};
use crate::util;

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

pub fn get(client: &ApiClient, _args: &ArgMatches) -> error::Result {
    let result = QuerySessionSecrets::new()
        .send(client)
        .context("failed to retrieve session secrets")?
        .into_payload();

    for secret in result {
        println!("{:?}", secret);
    }

    Ok(())
}

pub fn update(client: &ApiClient, _args: &ArgMatches) -> error::Result {
    CreateSessionSecret::new()
        .send(client)
        .context("failed to create session secret")?;

    Ok(())
}

pub fn remove(client: &ApiClient, _args: &ArgMatches) -> error::Result {
    DeleteSessionSecret::amount(1)
        .send(client)
        .context("failed to remove session secret")?;

    Ok(())
}
