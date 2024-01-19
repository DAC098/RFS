use rfs_api::client::ApiClient;
use rfs_api::client::users::{
    CreateUser,
    UpdateUser,
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::util;

mod group;

#[derive(Debug, Args)]
pub struct UsersArgs {
    #[command(subcommand)]
    command: UsersCmds
}

#[derive(Debug, Subcommand)]
enum UsersCmds {
    /// creats a new user
    Create(CreateArgs),

    /// updates a user
    Update(UpdateArgs),

    /// interacts with users for a group
    Groups(group::GroupsArgs),
}

pub fn handle(client: &ApiClient, args: UsersArgs) -> error::Result {
    match args.command {
        UsersCmds::Create(given) => create(client, given),
        UsersCmds::Update(given) => update(client, given),
        UsersCmds::Groups(given) => group::handle(client, given),
    }
}

#[derive(Debug, Args)]
struct CreateArgs {
    /// username of new user
    #[arg(long)]
    username: String,

    /// email of new user
    #[arg(long)]
    email: Option<String>
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result<()> {
    let mut builder = CreateUser::username(args.username);

    if let Some(email) = args.email {
        builder.email(email);
    }

    let result = builder.send(client)
        .context("failed to create new user")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// id of the user to update
    #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::UserId>))]
    id: rfs_lib::ids::UserId,

    /// updates username
    #[arg(long)]
    username: Option<String>,

    /// updates email
    #[arg(long, conflicts_with("no_email"))]
    email: Option<String>,

    /// removes the email
    #[arg(long, conflicts_with("email"))]
    no_email: bool
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result<()> {
    let mut builder = UpdateUser::id(args.id);

    if let Some(username) = args.username {
        builder.username(username);
    }

    if let Some(given) = args.email {
        builder.email(Some(given));
    } else if args.no_email {
        builder.email(None::<String>);
    }

    let result = builder.send(client)
        .context("failed to update desired user")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}
