use rfs_api::client::ApiClient;
use clap::{Command, ArgMatches};

use crate::error;
use crate::util;

mod totp;

pub fn command() -> Command {
    Command::new("auth")
        .subcommand_required(true)
        .about("interacts with auth data for the current user")
        .arg(util::default_help_arg())
        .subcommand(totp::command())
}

pub fn totp(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", _)) => totp::get(client),
        Some(("enable", enable_matches)) => totp::enable(client, enable_matches),
        Some(("disable", _)) => totp::disable(client),
        Some(("update", update_matches)) => totp::update(client, update_matches),
        Some(("recovery", recovery_matches)) => totp::recovery(client, recovery_matches),
        _ => unreachable!()
    }
}
