use rfs_api::client::ApiClient;
use clap::{Command, ArgMatches};

use crate::error;
use crate::util;

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

pub fn secrets(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("password", password_matches)) => secrets::password(client, password_matches),
        Some(("session", session_matches)) => secrets::session(client, session_matches),
        _ => unreachable!()
    }
}

pub fn roles(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => roles::get(client, get_matches),
        Some(("create", create_matches)) => roles::create(client, create_matches),
        Some(("update", update_matches)) => roles::update(client, update_matches),
        Some(("delete", delete_matches)) => roles::delete(client, delete_matches),
        Some(("users", users_matches)) => roles::users(client, users_matches),
        Some(("groups", groups_matches)) => roles::groups(client, groups_matches),
        _ => unreachable!()
    }
}
