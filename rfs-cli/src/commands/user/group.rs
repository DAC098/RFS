use rfs_api::client::ApiClient;
use rfs_api::client::users::groups::{
    QueryGroups,
    RetrieveGroup,
    CreateGroup,
    UpdateGroup,
    DeleteGroup,
    QueryGroupUsers,
    AddUsers,
    DropUsers,
};
use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

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

pub fn get(client: &ApiClient, args: &ArgMatches) -> error::Result {
    if let Some(group_id) = args.get_one::<i64>("id") {
        let result = RetrieveGroup::id(*group_id)
            .send(client)
            .context("failed to retrieve group")?;

        if let Some(payload) = result {
            println!("{:#?}", payload.into_payload());
        } else {
            println!("group not found");
        }
    } else {
        let result = QueryGroups::new()
            .send(client)
            .context("failed to retrieve groups")?
            .into_payload();

        for group in result {
            println!("{:#?}", group);
        }
    }

    Ok(())
}

pub fn create(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let name: String = args.get_one("name").cloned().unwrap();

    let result = CreateGroup::name(name)
        .send(client)
        .context("failed to create new group")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

pub fn update(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();
    let name = args.get_one::<String>("name").cloned().unwrap();

    let result = UpdateGroup::id(*id, name)
        .send(client)
        .context("failed to update group")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

pub fn delete(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();

    let result = DeleteGroup::id(*id)
        .send(client)
        .context("failed to delete group")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

pub fn users(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => get_users(client, get_matches),
        Some(("add", add_matches)) => add_users(client, add_matches),
        Some(("drop", drop_matches)) => drop_users(client, drop_matches),
        _ => unreachable!()
    }
}

fn get_users(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();

    let result = QueryGroupUsers::id(*id)
        .send(client)
        .context("failed to retrieve group users")?;

    if let Some(payload) = result {
        for user in payload.into_payload() {
            println!("{:#?}", user);
        }
    } else {
        println!("group not found");
    }

    Ok(())
}

fn get_user_ids(args: &ArgMatches) -> error::Result<Vec<rfs_lib::ids::UserId>> {
    let mut rtn = Vec::new();

    let Some(list) = args.get_many::<i64>("user") else {
        return Ok(rtn);
    };

    for id in list {
        let flake = id.try_into().context("invalid user id format")?;

        rtn.push(flake);
    }

    Ok(rtn)
}

fn add_users(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();

    let mut builder = AddUsers::id(*id);
    builder.add_iter(get_user_ids(args)?);
    builder.send(client)
        .context("failed to add users to group")?;

    Ok(())
}

fn drop_users(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<i64>("id").unwrap();

    let mut builder = DropUsers::id(*id);
    builder.add_iter(get_user_ids(args)?);
    builder.send(client)
        .context("failed to drop users from group")?;

    Ok(())
}
