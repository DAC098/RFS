use rfs_api::client::ApiClient;
use clap::{Command, ArgMatches};

use crate::error;
use crate::util;

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

pub fn password(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => password::get(client, get_matches),
        Some(("update", _update_matches)) => password::update(client, _update_matches),
        Some(("remove", _remove_matches)) => password::remove(client, _remove_matches),
        _ => unreachable!()
    }
}

pub fn session(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", _get_matches)) => session::get(client, _get_matches),
        Some(("update", _update_matches)) => session::update(client, _update_matches),
        Some(("remove", _remove_matches)) => session::remove(client, _remove_matches),
        _ => unreachable!()
    }
}
