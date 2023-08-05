use clap::{Command, ArgMatches};

use crate::error;
use crate::util;
use crate::state::AppState;

mod totp;

pub fn command() -> Command {
    Command::new("auth")
        .subcommand_required(true)
        .about("interacts with auth data for the current user")
        .arg(util::default_help_arg())
        .subcommand(totp::command())
}

pub fn totp(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => totp::get(state, get_matches),
        Some(("enable", enable_matches)) => totp::enable(state, enable_matches),
        Some(("disable", disable_matches)) => totp::disable(state, disable_matches),
        Some(("update", update_matches)) => totp::update(state, update_matches),
        Some(("recovery", recovery_matches)) => totp::recovery(state, recovery_matches),
        _ => unreachable!()
    }
}
