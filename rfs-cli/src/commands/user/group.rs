use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error;
use crate::util;
use crate::state::AppState;

pub fn command() -> Command {
    Command::new("group")
        .subcommand_required(true)
        .about("interactions with user groups on a server")
        .arg(util::default_help_arg())
        .subcommand(Command::new("get")
            .about("retrieves a list of groups or a specific group")
            .arg(util::default_help_arg())
            .arg(Arg::new("id")
                .long("id")
                .value_parser(value_parser!(i64))
                .help("id of the specific group to retrieve data for")
            )
        )
        .subcommand(Command::new("create")
            .about("creates a new group")
            .arg(util::default_help_arg())
            .arg(Arg::new("name")
                .long("name")
                .required(true)
                .help("specifies the name of the new user group")
            )
        )
        .subcommand(Command::new("update")
            .about("updates a given group")
            .arg(util::default_help_arg())
            .arg(Arg::new("id")
                .long("id")
                .value_parser(value_parser!(i64))
                .required(true)
                .help("id of the specific group to update")
            )
            .arg(Arg::new("name")
                .long("name")
                .required(true)
                .help("the new name to give the group")
            )
        )
        .subcommand(Command::new("delete")
            .about("deletes a given group")
            .arg(util::default_help_arg())
            .arg(Arg::new("id")
                .long("id")
                .value_parser(value_parser!(i64))
                .required(true)
                .help("id of the specific group to delete")
            )
        )
        .subcommand(Command::new("users")
            .subcommand_required(true)
            .about("interactions with users attahced to groups")
            .arg(util::default_help_arg())
            .subcommand(Command::new("get")
                .about("retrieves a list of users in a group")
                .arg(util::default_help_arg())
                .arg(Arg::new("id")
                    .long("id")
                    .value_parser(value_parser!(i64))
                    .required(true)
                    .help("id of the specific group to retrieve data for")
                )
            )
            .subcommand(Command::new("add")
                .about("adds users to a group")
                .arg(util::default_help_arg())
                .arg(Arg::new("id")
                    .long("id")
                    .value_parser(value_parser!(i64))
                    .required(true)
                    .help("id of the specific group to add users to")
                )
                .arg(Arg::new("user")
                    .short('u')
                    .long("user")
                    .action(ArgAction::Append)
                    .value_parser(value_parser!(i64))
                    .help("user ids to add to the group")
                )
            )
            .subcommand(Command::new("drop")
                .about("drops users from a group")
                .arg(util::default_help_arg())
                .arg(Arg::new("id")
                    .long("id")
                    .value_parser(value_parser!(i64))
                    .required(true)
                    .help("id of the specific group to drop users from")
                )
                .arg(Arg::new("user")
                    .short('u')
                    .long("user")
                    .action(ArgAction::Append)
                    .value_parser(value_parser!(i64))
                    .help("user ids to drop from the group")
                )
            )
        )
}

pub fn get(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let given_id: bool;
    let path = if let Some(id) = args.get_one::<i64>("id") {
        given_id = true;
        format!("/user/group/{}", id)
    } else {
        given_id = false;
        format!("/user/group")
    };

    let url = state.server.url.join(&path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedGroupLookup")
            .message("failed to lookup desired group information")
            .source(json));
    }

    if given_id {
        let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::user::group::Group>>()?;

        println!("{:#?}", result);
    } else {
        let result = res.json::<rfs_lib::json::ListWrapper<Vec<rfs_lib::schema::user::group::ListItem>>>()?;

        println!("{:#?}", result);
    }

    Ok(())
}

pub fn create(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let action = rfs_lib::actions::user::group::CreateGroup {
        name: args.get_one("name").cloned().unwrap()
    };

    let url = state.server.url.join("/user/group")?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedCreatingGroup")
            .message("failed to create the desired group")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::user::group::Group>>()?;

    println!("{:#?}", result);

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();
    let action = rfs_lib::actions::user::group::UpdateGroup {
        name: args.get_one("name").cloned().unwrap()
    };

    let path = format!("/user/group/{}", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.patch(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedUpdatingGroup")
            .message("failed to update the desired group")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::user::group::Group>>()?;

    println!("{:#?}", result);

    Ok(())
}

pub fn delete(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();

    let path = format!("/user/group/{}", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.delete(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedDeletingGroup")
            .message("failed to delete the desired group")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::user::group::Group>>()?;

    println!("{:#?}", result);

    Ok(())
}

pub fn users(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => get_users(state, get_matches),
        Some(("add", add_matches)) => add_users(state, add_matches),
        Some(("drop", drop_matches)) => drop_users(state, drop_matches),
        _ => unreachable!()
    }
}

fn get_users(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();

    let path = format!("/user/group/{}/users", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedGroupUsersLookup")
            .message("failed to retrieve a list of users in the desired group")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::ListWrapper<Vec<rfs_lib::schema::user::group::GroupUser>>>()?;

    println!("{:#?}", result);

    Ok(())
}

fn get_user_ids(args: &ArgMatches) -> error::Result<Vec<rfs_lib::ids::UserId>> {
    let mut rtn = Vec::new();

    let Some(list) = args.get_many::<i64>("user") else {
        return Ok(rtn);
    };

    for id in list {
        let flake = id.try_into()
            .map_err(|e| error::Error::new()
                .kind("InvalidUserId")
                .message("a provided user id is not a valid format")
                .source(e))?;

        rtn.push(flake);
    }

    Ok(rtn)
}

fn add_users(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();
    let action = rfs_lib::actions::user::group::AddUsers { 
        ids: get_user_ids(args)?
    };

    let path = format!("/user/group/{}/users", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedAddingGroupUsers")
            .message("failed to add new users to the desired group")
            .source(json));
    }

    Ok(())
}

fn drop_users(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();
    let action = rfs_lib::actions::user::group::DropUsers {
        ids: get_user_ids(args)?
    };

    let path = format!("/user/group/{}/users", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.delete(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedDroppingGroupUsers")
            .message("failed to drop users from the desired group")
            .source(json));
    }

    Ok(())
}
