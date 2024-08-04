use std::collections::HashSet;

use rfs_lib::ids;
use rfs_api::client::ApiClient;
use rfs_api::client::iterate;
use rfs_api::client::sec::roles::{
    AddRoleGroups,
    AddRoleUsers,
    CreateRole,
    DeleteRole,
    DropRoleGroups,
    DropRoleUsers,
    QueryRoleGroups,
    QueryRoleUsers,
    QueryRoles,
    RetrieveRole,
    UpdateRole
};
use rfs_lib::sec::authz::permission::{Scope, Ability};

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::formatting::{TextTable, Column, Float, PRETTY_OPTIONS};

#[derive(Debug, Args)]
pub struct RolesArgs {
    #[command(flatten)]
    get: GetArgs,

    #[command(subcommand)]
    command: Option<RolesCmds>,
}

#[derive(Debug, Subcommand)]
enum RolesCmds {
    /// creates a new role
    Create(CreateArgs),

    /// updates a role
    Update(UpdateArgs),

    /// delets a role
    Delete(DeleteArgs),

    /// interacts with users attached to a role
    Users(UsersArgs),

    /// interacts with groups attached to a role
    Groups(GroupsArgs),
}

pub fn handle(client: &ApiClient, args: RolesArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            RolesCmds::Create(given) => create(client, given),
            RolesCmds::Update(given) => update(client, given),
            RolesCmds::Delete(given) => delete(client, given),
            RolesCmds::Users(given) => handle_users(client, given),
            RolesCmds::Groups(given) => handle_groups(client, given),
        }
    } else {
        get(client, args.get)
    }
}

#[derive(Debug, Args)]
struct GetArgs {
    /// uid of the role to retrieve
    #[arg(long)]
    uid: Option<ids::RoleUid>,
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    if let Some(uid) = args.uid {
        let result = RetrieveRole::uid(uid)
            .send(client)
            .context("failed to retrieve role")?;

        if let Some(payload) = result {
            let inner = payload.into_payload();
            let mut table = TextTable::with_columns([
                Column::builder("scope").build(),
                Column::builder("ability").build()
            ]);

            println!("uid: {}\nname: \"{}\"", inner.uid, inner.name);

            for perm in inner.permissions {
                let mut row = table.add_row();
                row.set_col(0, perm.scope.as_str());
                row.set_col(1, perm.ability.as_str());

                row.finish_sort(perm);
            }

            if !table.is_empty() {
                table.print(&PRETTY_OPTIONS)
                    .context("failed to output results to stdout")?;
            }
        } else {
            println!("role not found");
        }
    } else {
        let mut builder = QueryRoles::new();
        let mut table = TextTable::with_columns([
            Column::builder("uid").float(Float::Right).build(),
            Column::builder("name").build(),
        ]);

        for result in iterate::Iterate::new(client, &mut builder) {
            let role = result.context("failed to retrieve roles")?;
            let mut row = table.add_row();
            row.set_col(0, role.uid.clone());
            row.set_col(1, role.name.clone());

            row.finish(role);
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

fn parse_permission(arg: &str) -> Result<(Scope, Ability), String> {
    let Some((scope, ability)) = arg.split_once(':') else {
        return Err("invalid permission format".into());
    };

    let Some(s) = Scope::from_str(scope) else {
        return Err("invalid scope value".into());
    };
    let Some(a) = Ability::from_str(ability) else {
        return Err("invalid ability value".into());
    };

    Ok((s,a))
}

#[derive(Debug, Args)]
struct CreateArgs {
    /// name of the new role
    #[arg(long)]
    name: String,

    /// permissions to add to new role
    #[arg(long, value_parser(parse_permission))]
    perm: Vec<(Scope, Ability)>,
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result {
    let mut builder = CreateRole::name(args.name);
    builder.add_iter_permission(args.perm);

    let result = builder.send(client)
        .context("failed to create role")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// id of the role to update
    #[arg(long)]
    uid: ids::RoleUid,

    /// updates name of role
    #[arg(short, long)]
    name: Option<String>,

    /// overrides current permissions
    #[arg(
        long,
        conflicts_with_all(["add_perm", "drop_perm"]),
        value_parser(parse_permission)
    )]
    perm: Vec<(Scope, Ability)>,

    /// add to existing permissions
    #[arg(
        long,
        conflicts_with("perm"),
        value_parser(parse_permission)
    )]
    add_perm: Vec<(Scope, Ability)>,

    /// drops from existing permissions
    #[arg(
        long,
        conflicts_with("perm"),
        value_parser(parse_permission)
    )]
    drop_perm: Vec<(Scope, Ability)>,
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result {
    let result = RetrieveRole::uid(args.uid.clone())
        .send(client)
        .context("failed to retrieve role")?;

    let current = {
        let Some(payload) = result else {
            println!("role not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateRole::uid(args.uid);

    if let Some(name) = args.name {
        builder.name(name);
    }

    if args.perm.len() > 0 {
        builder.add_iter_permissions(args.perm);
    } else if args.drop_perm.len() > 0 || args.add_perm.len() > 0 {
        let mut current_permissions: HashSet<(Scope, Ability)> = HashSet::from_iter(
            current.permissions.into_iter().map(|perm| (perm.scope, perm.ability))
        );

        for key in args.drop_perm {
            current_permissions.remove(&key);
        }

        for key in args.add_perm {
            current_permissions.insert(key);
        }

        builder.add_iter_permissions(current_permissions);
    }

    let result = builder.send(client)
        .context("failed to update role")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct DeleteArgs {
    /// id of the role to delete
    #[arg(long)]
    uid: ids::RoleUid,
}

fn delete(client: &ApiClient, args: DeleteArgs) -> error::Result {
    DeleteRole::uid(args.uid)
        .send(client)
        .context("failed to delete role")?;

    Ok(())
}

#[derive(Debug, Args)]
struct UsersArgs {
    #[command(flatten)]
    get: GetUsersArgs,

    #[command(subcommand)]
    command: Option<UsersCmds>,
}

#[derive(Debug, Subcommand)]
enum UsersCmds {
    /// adds users to a role
    #[command(name = "add")]
    AddUsers(AddUsersArgs),

    /// drops users from a role
    #[command(name = "drop")]
    DropUsers(DropUsersArgs),
}

fn handle_users(client: &ApiClient, args: UsersArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            UsersCmds::AddUsers(given) => add_users(client, given),
            UsersCmds::DropUsers(given) => drop_users(client, given),
        }
    } else {
        get_users(client, args.get)
    }
}

#[derive(Debug, Args)]
struct GetUsersArgs {
    /// id of the role
    #[arg(long)]
    uid: ids::RoleUid,
}

fn get_users(client: &ApiClient, args: GetUsersArgs) -> error::Result {
    let mut builder = QueryRoleUsers::uid(args.uid);
    let mut table = TextTable::with_columns([
        Column::builder("id").float(Float::Right).build(),
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
    /// id fof the role
    #[arg(long)]
    uid: ids::RoleUid,

    /// user ids to add
    #[arg(long)]
    user: Vec<ids::UserUid>,
}

fn add_users(client: &ApiClient, args: AddUsersArgs) -> error::Result {
    let mut builder = AddRoleUsers::uid(args.uid);
    builder.add_iter_id(args.user);
    builder.send(client)
        .context("failed to add users to role")?;

    Ok(())
}

#[derive(Debug, Args)]
struct DropUsersArgs {
    /// id of the role
    #[arg(long)]
    uid: ids::RoleUid,

    /// user ids to drop
    #[arg(long)]
    user: Vec<ids::UserUid>
}

fn drop_users(client: &ApiClient, args: DropUsersArgs) -> error::Result {
    let mut builder = DropRoleUsers::uid(args.uid);
    builder.add_iter_id(args.user);
    builder.send(client)
        .context("failed to drop users from role")?;

    Ok(())
}

#[derive(Debug, Args)]
struct GroupsArgs {
    #[command(flatten)]
    get: GetGroupsArgs,

    #[command(subcommand)]
    command: Option<GroupsCmds>,
}

#[derive(Debug, Subcommand)]
enum GroupsCmds {
    /// adds groups to a role
    #[command(name = "add")]
    AddGroups(AddGroupsArgs),

    /// drops groups from a role
    #[command(name = "drop")]
    DropGroups(DropGroupsArgs),
}

fn handle_groups(client: &ApiClient, args: GroupsArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            GroupsCmds::AddGroups(given) => add_groups(client, given),
            GroupsCmds::DropGroups(given) => drop_groups(client, given),
        }
    } else {
        get_groups(client, args.get)
    }
}

#[derive(Debug, Args)]
struct GetGroupsArgs {
    /// uid of the role
    #[arg(long)]
    uid: ids::RoleUid,
}

fn get_groups(client: &ApiClient, args: GetGroupsArgs) -> error::Result {
    let mut builder = QueryRoleGroups::uid(args.uid);
    let mut table = TextTable::with_columns([
        Column::builder("uid").float(Float::Right).build(),
    ]);

    for result in iterate::Iterate::new(client, &mut builder) {
        let group = result.context("failed to retrieve role groups")?;
        let mut row = table.add_row();
        row.set_col(0, group.uid.clone());

        row.finish(group);
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
struct AddGroupsArgs {
    /// id of the role
    #[arg(long)]
    uid: ids::RoleUid,

    /// groups to add
    #[arg(long)]
    group: Vec<ids::GroupUid>
}

fn add_groups(client: &ApiClient, args: AddGroupsArgs) -> error::Result {
    let mut builder = AddRoleGroups::uid(args.uid);
    builder.add_iter_id(args.group);
    builder.send(client)
        .context("failed to add groups to role")?;

    Ok(())
}

#[derive(Debug, Args)]
struct DropGroupsArgs {
    /// uid of role
    #[arg(long)]
    uid: ids::RoleUid,

    /// groups to drop
    #[arg(long)]
    group: Vec<ids::GroupUid>
}

fn drop_groups(client: &ApiClient, args: DropGroupsArgs) -> error::Result {
    let mut builder = DropRoleGroups::uid(args.uid);
    builder.add_iter_id(args.group);
    builder.send(client)
        .context("failed to drop groups from role")?;

    Ok(())
}
