use rfs_api::client::ApiClient;
use rfs_api::client::users::{
    CreateUser,
    UpdateUser,
};

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

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

pub fn group(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => group::get(client, get_matches),
        Some(("create", create_matches)) => group::create(client, create_matches),
        Some(("update", update_matches)) => group::update(client, update_matches),
        Some(("delete", delete_matches)) => group::delete(client, delete_matches),
        Some(("users", users_matches)) => group::users(client, users_matches),
        _ => unreachable!()
    }
}

pub fn create(client: &ApiClient, args: &ArgMatches) -> error::Result<()> {
    let username: String = args.get_one("username").cloned().unwrap();
    let email: Option<String> = args.get_one("email").cloned();

    let mut builder = CreateUser::username(username);

    if let Some(email) = email {
        builder.email(email);
    }

    let result = builder.send(client)
        .context("failed to create new user")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

pub fn update(client: &ApiClient, args: &ArgMatches) -> error::Result<()> {
    let id: i64 = args.get_one("id").cloned().unwrap();

    let user_id = id.try_into().context("invalid user id format")?;
    let mut builder = UpdateUser::id(user_id);

    if let Some(username) = args.get_one::<String>("username") {
        builder.username(username);
    }

    if args.get_flag("no-email") {
        builder.email(None::<String>);
    } else if let Some(given) = args.get_one::<String>("email") {
        builder.email(Some(given));
    }

    let result = builder.send(client)
        .context("failed to update desired user")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}
