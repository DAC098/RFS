use rfs_api::client::ApiClient;
use rfs_api::client::sec::secrets::{
    CreatePasswordSecret,
    QueryPasswordSecrets,
    RetrievePasswordSecret,
    DeletePasswordSecret
};

use clap::{Command, Arg, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

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

pub fn get(client: &ApiClient, args: &ArgMatches) -> error::Result {
    if let Some(version) = args.get_one::<u64>("version") {
        let result = RetrievePasswordSecret::version(*version)
            .send(client)
            .context("failed to retrieve password secret")?;

        if let Some(payload) = result {
            println!("{:?}", payload.into_payload());
        } else {
            println!("password secret not found");
        }
    } else {
        let result = QueryPasswordSecrets::new()
            .send(client)
            .context("failed to retrieve password secrets")?
            .into_payload();

        for secret in result {
            println!("{:?}", secret);
        }
    }

    Ok(())
}

pub fn update(client: &ApiClient, _args: &ArgMatches) -> error::Result {
    CreatePasswordSecret::new()
        .send(client)
        .context("failed to create password secret")?;

    Ok(())
}

pub fn remove(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let version = args.get_one::<u64>("version").unwrap();

    DeletePasswordSecret::version(*version)
        .send(client)
        .context("failed to remove password secret")?;

    Ok(())
}
