use clap::{Command, ArgMatches};

use crate::error;
use crate::util;
use crate::state::AppState;

mod password;
mod session;

pub fn command() -> Command {
    Command::new("secrets")
        .subcommand_required(true)
        .about("helps to mange secrets on the server")
        .arg(util::default_help_arg())
        .subcommand(password::command())
        .subcommand(session::command())
}

pub fn password(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => password::get(state, get_matches),
        Some(("update", _update_matches)) => password::update(state, _update_matches),
        Some(("remove", _remove_matches)) => password::remove(state, _remove_matches),
        _ => unreachable!()
    }
}

pub fn session(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", _get_matches)) => session::get(state, _get_matches),
        Some(("update", _update_matches)) => session::update(state, _update_matches),
        Some(("remove", _remove_matches)) => session::remove(state, _remove_matches),
        _ => unreachable!()
    }
}
