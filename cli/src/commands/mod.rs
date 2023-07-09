use std::path::PathBuf;

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error;
use crate::input;
use crate::auth;
use crate::util;
use crate::state::AppState;

mod storage;

fn append_subcommands(command: Command) -> Command {
    command
        .subcommand(storage::command())
        .subcommand(Command::new("connect")
            .alias("login")
            .about("logs in into the desired server")
        )
        .subcommand(Command::new("disconnect")
            .alias("logout")
            .about("logs out of the desired server")
        )
}

pub fn cli() -> Command {
    let command = Command::new("rfs-cli")
        .disable_help_flag(true)
        .arg(util::default_help_arg())
        .arg(Arg::new("cookies")
            .long("cookies")
            .action(ArgAction::Set)
            .value_parser(value_parser!(PathBuf))
            .help("specifies a specific file to save any cookie data to")
        )
        .arg(Arg::new("host")
            .long("host")
            .short('h')
            .action(ArgAction::Set)
            .default_value("localhost")
            .help("the desired hostname to connect to")
        )
        .arg(Arg::new("port")
            .long("port")
            .short('p')
            .action(ArgAction::Set)
            .default_value("80")
            .value_parser(value_parser!(u16))
            .help("the desired port to connect to")
        )
        .arg(Arg::new("secure")
            .long("secure")
            .short('s')
            .action(ArgAction::SetTrue)
            .help("sets the connection to use https")
        );

    append_subcommands(command)
}

pub fn interactive() -> Command {
    let command = Command::new("")
        .subcommand_required(true)
        .no_binary_name(true)
        .disable_help_flag(true)
        .arg(util::default_help_arg())
        .subcommand(Command::new("quit")
            .alias("q")
            .about("exits program")
        );

    append_subcommands(command)
}

pub fn connect(state: &mut AppState, _args: &ArgMatches) -> error::Result<()> {
    auth::connect(state)?;

    println!("session authenticated");

    Ok(())
}

pub fn disconnect(state: &mut AppState, _args: &ArgMatches) -> error::Result<()> {
    auth::disconnect(state)?;

    Ok(())
}

pub fn storage(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    match args.subcommand() {
        Some(("create", create_args)) => storage::create(state, create_args)?,
        Some(("update", update_args)) => storage::update(state, update_args)?,
        _ => unreachable!()
    }

    Ok(())
}

