use rfs_api::client::ApiClient;
use rfs_api::client::users::{
    QueryUsers,
    CreateUser,
    UpdateUser,
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::input;
use crate::util;

mod group;

#[derive(Debug, Args)]
pub struct UsersArgs {
    #[command(subcommand)]
    command: UsersCmds
}

#[derive(Debug, Subcommand)]
enum UsersCmds {
    /// retrieves a list of users
    Get(GetArgs),

    /// creats a new user
    Create(CreateArgs),

    /// updates a user
    Update(UpdateArgs),

    /// interacts with users for a group
    Groups(group::GroupsArgs),
}

pub fn handle(client: &ApiClient, args: UsersArgs) -> error::Result {
    match args.command {
        UsersCmds::Get(given) => get(client, given),
        UsersCmds::Create(given) => create(client, given),
        UsersCmds::Update(given) => update(client, given),
        UsersCmds::Groups(given) => group::handle(client, given),
    }
}

#[derive(Debug, Args)]
struct GetArgs {
    /// will retrieve all values and not prompt for more
    #[arg(long)]
    no_prompt: bool
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    let mut builder = QueryUsers::new();

    loop {
        let (_pagination, payload) = builder.send(client)
            .context("failed to retrieve users")?
            .into_tuple();

        let Some(last) = payload.last() else {
            break;
        };

        builder.last_id(last.id.clone());

        for user in &payload {
            println!("id: {} | username: {}", user.id.id(), user.username);
        }

        if !args.no_prompt && !input::read_yn("continue?")? {
            break;
        }
    }

    Ok(())
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
