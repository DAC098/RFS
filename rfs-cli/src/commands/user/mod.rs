use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error;
use crate::util;
use crate::state::AppState;

mod group;

pub fn command() -> Command {
    Command::new("user")
        .subcommand_required(true)
        .about("interacts with user data on a server")
        .arg(util::default_help_arg())
        .subcommand(group::command())
        .subcommand(Command::new("create")
            .about("creates a new user")
            .arg(util::default_help_arg())
            .arg(Arg::new("username")
                .long("username")
                .required(true)
                .help("specifies the username of the new user")
            )
            .arg(Arg::new("email")
                .long("email")
                .help("specifies the email of the new user")
            )
        )
        .subcommand(Command::new("update")
            .about("updates the desired user with new information")
            .arg(util::default_help_arg())
            .arg(Arg::new("id")
                .long("id")
                .value_parser(value_parser!(i64))
                .required(true)
                .help("id of the user to update with new data")
            )
            .arg(Arg::new("username")
                .long("username")
                .help("new username for the desired user")
            )
            .arg(Arg::new("email")
                .long("email")
                .help("new email for the desired user")
                .conflicts_with("no-email")
            )
            .arg(Arg::new("no-email")
                .long("no-email")
                .action(ArgAction::SetTrue)
                .help("removes the email for the desired user")
                .conflicts_with("email")
            )
        )
}

pub fn group(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => group::get(state, get_matches),
        Some(("create", create_matches)) => group::create(state, create_matches),
        Some(("update", update_matches)) => group::update(state, update_matches),
        Some(("delete", delete_matches)) => group::delete(state, delete_matches),
        Some(("users", users_matches)) => group::users(state, users_matches),
        _ => unreachable!()
    }
}

pub fn create(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let username: String = args.get_one("username").cloned().unwrap();
    let email: Option<String> = args.get_one("email").cloned();

    let action = rfs_lib::actions::user::CreateUser {
        username,
        email
    };

    let url = state.server.url.join("/user")?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedCreatingUser")
            .message("failed to create the new user")
            .source(format!("{:?}", json)));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::user::User>>()?;

    println!("{:?}", result.into_payload());

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let id: i64 = args.get_one("id").cloned().unwrap();
    let path = format!("/user/{}", id);

    let _user = {
        let url = state.server.url.join(&path)?;
        let res = state.client.get(url).send()?;

        let status = res.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(error::Error::new()
                .kind("UserNotFound")
                .message("the requested user was not found"));
        } else if status != reqwest::StatusCode::OK {
            let json = res.json::<rfs_lib::json::Error>()?;

            return Err(error::Error::new()
                .kind("FailedUserLookup")
                .message("failed to the the desired user")
                .source(format!("{:?}", json)));
        }

        let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::user::User>>()?;

        result.into_payload()
    };

    let email = if args.get_flag("no-email") {
        Some(None)
    } else if let Some(given) = args.get_one::<String>("email") {
        Some(Some(given.clone()))
    } else {
        None
    };

    let action = rfs_lib::actions::user::UpdateUser {
        username: args.get_one("username").cloned(),
        email
    };

    if !action.has_work() {
        return Err(error::Error::new()
            .kind("NoWork")
            .message("no changes have been specified"));
    }

    let url = state.server.url.join(&path)?;
    let res = state.client.patch(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(error::Error::new()
                .kind("UserNotFound")
                .message("the requested user was not found"));
        } else {
            return Err(error::Error::new()
                .kind("FailedUpdatingUser")
                .message("failed to update the desired user")
                .source(format!("{:?}", json)));
        }
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::user::User>>()?;

    println!("{:?}", result.into_payload());

    Ok(())
}
