use rfs_api::client::ApiClient;
use clap::{Subcommand, Args};

use crate::error;

mod password;
mod session;

#[derive(Debug, Args)]
pub struct SecretsArgs {
    #[command(subcommand)]
    command: SecretsCmds
}

#[derive(Debug, Subcommand)]
enum SecretsCmds {
    /// interacts with password secrets
    Password(password::PasswordArgs),

    /// interacts with session secrets
    Session(session::SessionArgs),
}

pub fn handle(client: &ApiClient, args: SecretsArgs) -> error::Result {
    match args.command {
        SecretsCmds::Password(given) => password::handle(client, given),
        SecretsCmds::Session(given) => session::handle(client, given),
    }
}
