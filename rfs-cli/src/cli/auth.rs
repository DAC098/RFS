use rfs_api::client::ApiClient;
use clap::{Subcommand, Args};

use crate::error;

mod totp;

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    command: AuthCmds
}

#[derive(Debug, Subcommand)]
enum AuthCmds {
    /// interacts with data specific to totp 2FA
    Totp(totp::TotpArgs)
}

pub fn handle(client: &ApiClient, args: AuthArgs) -> error::Result {
    match args.command {
        AuthCmds::Totp(given) => totp::handle(client, given),
    }
}

