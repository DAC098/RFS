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

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::util;

#[derive(Debug, Args)]
pub struct GroupsArgs {
    #[command(subcommand)]
    command: GroupsCmds
}

#[derive(Debug, Subcommand)]
enum GroupsCmds {
    /// retrieves a list of groups or a specific group
    Get(GetArgs),

    /// creates a new group
    Create(CreateArgs),

    /// updates a group
    Update(UpdateArgs),

    /// deletes a group
    Delete(DeleteArgs),

    /// interacts with users attached to a group
    Users(UsersArgs),
}

pub fn handle(client: &ApiClient, args: GroupsArgs) -> error::Result {
    match args.command {
        GroupsCmds::Get(given) => get(client, given),
        GroupsCmds::Create(given) => create(client, given),
        GroupsCmds::Update(given) => update(client, given),
        GroupsCmds::Delete(given) => delete(client, given),
        GroupsCmds::Users(given) => handle_users(client, given),
    }
}

#[derive(Debug, Args)]
struct GetArgs {
    /// id of the group to retrieve
    #[arg(long)]
    id: Option<i64>
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    if let Some(group_id) = args.id {
        let result = RetrieveGroup::id(group_id)
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

#[derive(Debug, Args)]
struct CreateArgs {
    /// name of the group
    #[arg(short, long)]
    name: String
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result {
    let result = CreateGroup::name(args.name)
        .send(client)
        .context("failed to create new group")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// id of the group to update
    #[arg(long)]
    id: i64,

    /// updates group name
    #[arg(short, long)]
    name: String
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result {
    let result = UpdateGroup::id(args.id, args.name)
        .send(client)
        .context("failed to update group")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct DeleteArgs {
    /// id of the group to delete
    #[arg(long)]
    id: i64
}

fn delete(client: &ApiClient, args: DeleteArgs) -> error::Result {
    let result = DeleteGroup::id(args.id)
        .send(client)
        .context("failed to delete group")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct UsersArgs {
    #[command(subcommand)]
    command: UsersCmds
}

#[derive(Debug, Subcommand)]
enum UsersCmds {
    /// retrieves a list of users in a group
    #[command(name = "get")]
    GetUsers(GetUsersArgs),

    /// adds users to a group
    #[command(name = "add")]
    AddUsers(AddUsersArgs),

    /// drops users from a group
    #[command(name = "drop")]
    DropUsers(DropUsersArgs),
}

fn handle_users(client: &ApiClient, args: UsersArgs) -> error::Result {
    match args.command {
        UsersCmds::GetUsers(given) => get_users(client, given),
        UsersCmds::AddUsers(given) => add_users(client, given),
        UsersCmds::DropUsers(given) => drop_users(client, given),
    }
}

#[derive(Debug, Args)]
struct GetUsersArgs {
    /// id of the group
    #[arg(long)]
    id: i64,
}

fn get_users(client: &ApiClient, args: GetUsersArgs) -> error::Result {
    let result = QueryGroupUsers::id(args.id)
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

#[derive(Debug, Args)]
struct AddUsersArgs {
    /// id of the group
    #[arg(long)]
    id: i64,

    /// id of the user to add
    #[arg(short, long = "user", value_parser(util::parse_flake_id::<rfs_lib::ids::UserId>))]
    users: Vec<rfs_lib::ids::UserId>,
}

fn add_users(client: &ApiClient, args: AddUsersArgs) -> error::Result {
    let mut builder = AddUsers::id(args.id);
    builder.add_iter(args.users);
    builder.send(client)
        .context("failed to add users to group")?;

    Ok(())
}

#[derive(Debug, Args)]
struct DropUsersArgs {
    /// id of the group
    #[arg(long)]
    id: i64,

    /// id of the user to drop
    #[arg(short, long = "user", value_parser(util::parse_flake_id::<rfs_lib::ids::UserId>))]
    users: Vec<rfs_lib::ids::UserId>
}

fn drop_users(client: &ApiClient, args: DropUsersArgs) -> error::Result {
    let mut builder = DropUsers::id(args.id);
    builder.add_iter(args.users);
    builder.send(client)
        .context("failed to drop users from group")?;

    Ok(())
}
