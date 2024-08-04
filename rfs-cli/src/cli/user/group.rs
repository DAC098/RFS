use rfs_lib::ids;
use rfs_api::client::{ApiClient, iterate};
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
use crate::formatting::{TextTable, Column, Float, PRETTY_OPTIONS};

#[derive(Debug, Args)]
pub struct GroupsArgs {
    #[command(flatten)]
    get: GetArgs,

    #[command(subcommand)]
    command: Option<GroupsCmds>
}

#[derive(Debug, Subcommand)]
enum GroupsCmds {
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
    if let Some(cmd) = args.command {
        match cmd {
            GroupsCmds::Create(given) => create(client, given),
            GroupsCmds::Update(given) => update(client, given),
            GroupsCmds::Delete(given) => delete(client, given),
            GroupsCmds::Users(given) => handle_users(client, given),
        }
    } else {
        get(client, args.get)
    }
}

#[derive(Debug, Args)]
struct GetArgs {
    /// uid of the group to retrieve
    #[arg(long)]
    uid: Option<ids::GroupUid>,
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    if let Some(uid) = args.uid {
        let result = RetrieveGroup::uid(uid)
            .send(client)
            .context("failed to retrieve group")?;

        if let Some(payload) = result {
            println!("{:#?}", payload.into_payload());
        } else {
            println!("group not found");
        }
    } else {
        let mut builder = QueryGroups::new();
        let mut table = TextTable::with_columns([
            Column::builder("uid").float(Float::Right).build(),
            Column::builder("name").build(),
        ]);

        for result in iterate::Iterate::new(client, &mut builder) {
            let group = result.context("failed to retrieve groups")?;
            let mut row = table.add_row();
            row.set_col(0, group.uid.clone());
            row.set_col(1, group.name.clone());
            row.finish(group);
        }

        if table.is_empty() {
            println!("no contents");
        } else {
            table.print(&PRETTY_OPTIONS)
                .context("failed to output results to stdout")?;
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
    /// uid of the group to update
    #[arg(long)]
    uid: ids::GroupUid,

    /// updates group name
    #[arg(short, long)]
    name: String
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result {
    let result = UpdateGroup::uid(args.uid, args.name)
        .send(client)
        .context("failed to update group")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct DeleteArgs {
    /// uid of the group to delete
    #[arg(long)]
    uid: ids::GroupUid
}

fn delete(client: &ApiClient, args: DeleteArgs) -> error::Result {
    let result = DeleteGroup::uid(args.uid)
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
    /// uid of the group
    #[arg(long)]
    uid: ids::GroupUid,

    /// will retrieve all values and not prompt for more
    #[arg(long)]
    no_prompt: bool
}

fn get_users(client: &ApiClient, args: GetUsersArgs) -> error::Result {
    let mut builder = QueryGroupUsers::uid(args.uid);
    let mut table = TextTable::with_columns([
        Column::builder("uid").float(Float::Right).build(),
    ]);

    for result in iterate::Iterate::new(client, &mut builder) {
        let user = result.context("failed to retrieve group users")?;
        let mut row = table.add_row();
        row.set_col(0, user.uid.clone());
        row.finish(user);
    }

    if table.is_empty() {
        println!("no contents");
    } else {
        table.print(&PRETTY_OPTIONS)
            .context("failed to output results to stdout")?;
    }

    Ok(())
}

#[derive(Debug, Args)]
struct AddUsersArgs {
    /// uid of the group
    #[arg(long)]
    uid: ids::GroupUid,

    /// id of the user to add
    #[arg(short, long = "user")]
    users: Vec<ids::UserUid>,
}

fn add_users(client: &ApiClient, args: AddUsersArgs) -> error::Result {
    let mut builder = AddUsers::uid(args.uid);
    builder.add_iter(args.users);
    builder.send(client)
        .context("failed to add users to group")?;

    Ok(())
}

#[derive(Debug, Args)]
struct DropUsersArgs {
    /// uid of the group
    #[arg(long)]
    uid: ids::GroupUid,

    /// id of the user to drop
    #[arg(short, long = "user")]
    users: Vec<rfs_lib::ids::UserUid>
}

fn drop_users(client: &ApiClient, args: DropUsersArgs) -> error::Result {
    let mut builder = DropUsers::uid(args.uid);
    builder.add_iter(args.users);
    builder.send(client)
        .context("failed to drop users from group")?;

    Ok(())
}
