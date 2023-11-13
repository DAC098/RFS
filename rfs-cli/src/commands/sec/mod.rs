use clap::{Command, ArgMatches};

use crate::error;
use crate::util;
use crate::state::AppState;

mod secrets;
mod roles;

pub fn command() -> Command {
    Command::new("sec")
        .subcommand_required(true)
        .about("helps to manage security related features of a server")
        .arg(util::default_help_arg())
        .subcommand(secrets::command())
        .subcommand(roles::command())
}

pub fn secrets(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("password", password_matches)) => secrets::password(state, password_matches),
        Some(("session", session_matches)) => secrets::session(state, session_matches),
        _ => unreachable!()
    }
}

pub fn roles(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => roles::get(state, get_matches),
        Some(("create", create_matches)) => roles::create(state, create_matches),
        Some(("update", update_matches)) => roles::update(state, update_matches),
        Some(("delete", delete_matches)) => roles::delete(state, delete_matches),
        Some(("users", users_matches)) => roles::users(state, users_matches),
        Some(("groups", groups_matches)) => roles::groups(state, groups_matches),
        _ => unreachable!()
    }
}
