use rfs_api::client::ApiClient;
use clap::{Subcommand, Args};

use crate::error;

mod secrets;
mod roles;

#[derive(Debug, Args)]
pub struct SecArgs {
    #[command(subcommand)]
    command: SecCmds
}

#[derive(Debug, Subcommand)]
enum SecCmds {
    /// interacts with roles
    Roles(roles::RolesArgs),
    /// interacts with secrets
    Secrets(secrets::SecretsArgs),
}

pub fn handle(client: &ApiClient, args: SecArgs) -> error::Result {
    match args.command {
        SecCmds::Roles(given) => roles::handle(client, given),
        SecCmds::Secrets(given) => secrets::handle(client, given),
    }
}

