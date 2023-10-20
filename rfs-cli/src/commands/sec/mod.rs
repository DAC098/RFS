use clap::{Command, ArgMatches};

use crate::error;
use crate::util;
use crate::state::AppState;

mod secrets;

pub fn command() -> Command {
    Command::new("sec")
        .subcommand_required(true)
        .about("helps to manage security related features of a server")
        .arg(util::default_help_arg())
        .subcommand(secrets::command())
}

pub fn secrets(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("password", password_matches)) => secrets::password(state, password_matches),
        Some(("session", session_matches)) => secrets::session(state, session_matches),
        _ => unreachable!()
    }
}
