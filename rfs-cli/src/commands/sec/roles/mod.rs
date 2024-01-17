use std::collections::HashSet;

use rfs_api::client::ApiClient;
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

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

pub fn id_arg() -> Arg {
    Arg::new("id")
        .long("id")
        .value_parser(value_parser!(rfs_lib::ids::RoleId))
}

pub fn command() -> Command {
    Command::new("roles")
        .subcommand_required(true)
        .about("interactions with roles on a server")
        .arg(util::default_help_arg())
        .subcommand(Command::new("get")
            .about("retrieves a list of roles or a specific role")
            .arg(util::default_help_arg())
            .arg(id_arg().help("id of the specific role to retrieve data for"))
        )
        .subcommand(Command::new("create")
            .about("creates a new role")
            .arg(util::default_help_arg())
            .arg(Arg::new("name")
                .long("name")
                .required(true)
                .help("specifies the name of the new role")
            )
            .arg(Arg::new("permission")
                .long("permission")
                .alias("perm")
                .help("the specific permission to add to the role")
            )
        )
        .subcommand(Command::new("update")
            .about("updates a given role")
            .arg(util::default_help_arg())
            .arg(id_arg()
                .required(true)
                .help("id of the specific role to update")
            )
            .arg(Arg::new("name")
                .long("name")
                .help("the new name of give the role")
            )
            .arg(Arg::new("permission")
                .long("permission")
                .alias("perm")
                .action(ArgAction::Append)
                .help("permissions to apply to the role. overrides currently set roles")
                .conflicts_with_all(["add-permission", "drop-permission"])
            )
            .arg(Arg::new("add-permission")
                .long("add-permission")
                .alias("add-perm")
                .action(ArgAction::Append)
                .help("adds a permission to the given role")
                .conflicts_with("permission")
            )
            .arg(Arg::new("drop-permission")
                .long("drop-permission")
                .alias("drop-perm")
                .action(ArgAction::Append)
                .help("drops a permission from the given role")
                .conflicts_with("permission")
            )
        )
        .subcommand(Command::new("delete")
            .about("deletes a given role")
            .arg(util::default_help_arg())
            .arg(Arg::new("id")
                .long("id")
                .value_parser(value_parser!(rfs_lib::ids::RoleId))
                .required(true)
                .help("id of the specific role to deleteJ")
            )
        )
        .subcommand(Command::new("users")
            .subcommand_required(true)
            .about("modifies users attached to a role")
            .arg(util::default_help_arg())
            .subcommand(Command::new("get")
                .about("retrieves a list of users for the specified role")
                .arg(util::default_help_arg())
                .arg(id_arg()
                    .required(true)
                    .help("id of the specified role to get users for")
                )
            )
            .subcommand(Command::new("add")
                .about("adds users to the specified role")
                .arg(util::default_help_arg())
                .arg(id_arg()
                    .required(true)
                    .help("id of the specificd role to add users to")
                )
                .arg(Arg::new("user")
                    .short('u')
                    .long("user")
                    .action(ArgAction::Append)
                    .value_parser(value_parser!(i64))
                    .help("user ids to add to the role")
                )
            )
            .subcommand(Command::new("drop")
                .about("drops users from the specified role")
                .arg(util::default_help_arg())
                .arg(id_arg()
                    .required(true)
                    .help("id of the specified role to drop users from")
                )
                .arg(Arg::new("user")
                    .short('u')
                    .long("user")
                    .action(ArgAction::Append)
                    .value_parser(value_parser!(i64))
                    .help("user ids to drop from the role")
                )
            )
        )
        .subcommand(Command::new("groups")
            .subcommand_required(true)
            .about("modifies groups attached to a role")
            .arg(util::default_help_arg())
            .subcommand(Command::new("get")
                .about("retrieves a list of groups for the specified role")
                .arg(util::default_help_arg())
                .arg(id_arg()
                    .required(true)
                    .help("id of the specified role to get groups for")
                )
            )
            .subcommand(Command::new("add")
                .about("adds groups to the specified role")
                .arg(util::default_help_arg())
                .arg(id_arg()
                    .required(true)
                    .help("id of the specified role to add groups to")
                )
                .arg(Arg::new("group")
                    .short('g')
                    .long("group")
                    .action(ArgAction::Append)
                    .value_parser(value_parser!(rfs_lib::ids::GroupId))
                    .help("group ids to add to the role")
                )
            )
            .subcommand(Command::new("drop")
                .about("drops groups from the specified role")
                .arg(util::default_help_arg())
                .arg(id_arg()
                    .required(true)
                    .help("id of the specified role to drop groups from")
                )
                .arg(Arg::new("group")
                    .short('g')
                    .long("group")
                    .action(ArgAction::Append)
                    .value_parser(value_parser!(rfs_lib::ids::GroupId))
                    .help("group ids to drop from the role")
                )
            )
        )
}

pub fn get(client: &ApiClient, args: &ArgMatches) -> error::Result {
    if let Some(id) = args.get_one::<rfs_lib::ids::RoleId>("id") {
        let result = RetrieveRole::id(*id)
            .send(client)
            .context("failed to retrieve role")?;

        if let Some(payload) = result {
            println!("{:#?}", payload.into_payload());
        } else {
            println!("role not found");
        }
    } else {
        let result = QueryRoles::new()
            .send(client)
            .context("failed to retrieve roles")?
            .into_payload();

        for role in result {
            println!("{:#?}", role);
        }
    }

    Ok(())
}

#[inline]
fn perm_tuple_to_role_permission<T>(iter: T) -> Vec<rfs_api::sec::roles::Permission>
where
    T: IntoIterator<Item = (Scope, Ability)>
{
    iter.into_iter()
        .map(|(scope, ability)| rfs_api::sec::roles::Permission {
            scope,
            ability
        })
        .collect()
}

fn parse_permissions(name: &str, args: &ArgMatches) -> error::Result<HashSet<(Scope, Ability)>> {
    let Some(mut given) = args.get_many::<String>(name) else {
        return Ok(HashSet::new());
    };

    let mut check = HashSet::new();
    let mut invalid = Vec::new();
    let mut invalid_scope = Vec::new();
    let mut invalid_ability = Vec::new();

    while let Some(parse) = given.next() {
        let Some((scope, ability)) = parse.split_once(':') else {
            invalid.push(parse);
            continue;
        };

        let Some(s) = Scope::from_str(scope) else {
            invalid_scope.push(parse);
            continue;
        };
        let Some(a) = Ability::from_str(ability) else {
            invalid_ability.push(parse);
            continue;
        };

        check.insert((s, a));
    }

    if invalid.len() != 0 {
        return Err(error::Error::new()
            .context(format!("provided permissions are an invalid format: {:?}", invalid)));
    }

    if invalid_scope.len() != 0 {
        return Err(error::Error::new()
            .context(format!("provided permissions that have an invalid scope: {:?}", invalid_scope)));
    }

    if invalid_ability.len() != 0 {
        return Err(error::Error::new()
            .context(format!("provided permissions that have an invalid ability: {:?}", invalid_ability)));
    }

    Ok(check)
}

pub fn create(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let name = args.get_one::<String>("name").unwrap();
    let permissions = perm_tuple_to_role_permission(parse_permissions("permission", args)?);

    let mut builder = CreateRole::name(name);
    builder.add_iter_permission(permissions);

    let result = builder.send(client)
        .context("failed to create role")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

pub fn update(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();

    let permissions = parse_permissions("permission", args)?;
    let add_permissions = parse_permissions("add-permission", args)?;
    let drop_permissions = parse_permissions("drop-permission", args)?;

    let result = RetrieveRole::id(*id)
        .send(client)
        .context("failed to retrieve role")?;

    let current = {
        let Some(payload) = result else {
            println!("role not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateRole::id(*id);

    if let Some(name) = args.get_one::<String>("name") {
        builder.name(name);
    }

    if permissions.len() > 0 {
        builder.add_iter_permissions(perm_tuple_to_role_permission(permissions));
    } else if drop_permissions.len() > 0 || add_permissions.len() > 0 {
        let mut current_permissions: HashSet<(Scope, Ability)> = HashSet::from_iter(
            current.permissions.iter()
                .map(|perm| (perm.scope.clone(), perm.ability.clone()))
        );

        for key in drop_permissions {
            current_permissions.remove(&key);
        }

        for key in add_permissions {
            current_permissions.insert(key);
        }

        builder.add_iter_permissions(perm_tuple_to_role_permission(current_permissions));
    }

    let result = builder.send(client)
        .context("failed to update role")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}

pub fn delete(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();

    DeleteRole::id(*id)
        .send(client)
        .context("failed to delete role")?;

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

pub fn groups(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => get_groups(client, get_matches),
        Some(("add", add_matches)) => add_groups(client, add_matches),
        Some(("drop", drop_matches)) => drop_groups(client, drop_matches),
        _ => unreachable!()
    }
}

pub fn get_users(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();

    let result = QueryRoleUsers::id(*id)
        .send(client)
        .context("failed to retrieve role users")?;

    if let Some(payload) = result {
        for user in payload.into_payload() {
            println!("{:#?}", user);
        }
    } else {
        println!("role not found");
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

fn get_group_ids(args: &ArgMatches) -> error::Result<Vec<rfs_lib::ids::GroupId>> {
    let mut rtn = Vec::new();

    let Some(list) = args.get_many::<rfs_lib::ids::GroupId>("group") else {
        return Ok(rtn);
    };

    for id in list {
        if *id <= 0 {
            return Err(error::Error::new()
                .context("a provided group id is not a valid format"));
        }

        rtn.push(*id);
    }

    Ok(rtn)
}

pub fn add_users(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let user_ids = get_user_ids(args)?;

    let mut builder = AddRoleUsers::id(*id);
    builder.add_iter_id(user_ids);
    builder.send(client)
        .context("failed to add users to role")?;

    Ok(())
}

pub fn drop_users(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let user_ids = get_user_ids(args)?;

    let mut builder = DropRoleUsers::id(*id);
    builder.add_iter_id(user_ids);
    builder.send(client)
        .context("failed to drop users from role")?;

    Ok(())
}

pub fn get_groups(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();

    let result = QueryRoleGroups::id(*id)
        .send(client)
        .context("failed to retrieve role groups")?;

    if let Some(payload) = result {
        for group in payload.into_payload() {
            println!("{:#?}", group);
        }
    } else {
        println!("role not found");
    }

    Ok(())
}

pub fn add_groups(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let group_ids = get_group_ids(args)?;

    let mut builder = AddRoleGroups::id(*id);
    builder.add_iter_id(group_ids);
    builder.send(client)
        .context("failed to add groups to role")?;

    Ok(())
}

pub fn drop_groups(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let group_ids = get_group_ids(args)?;

    let mut builder = DropRoleGroups::id(*id);
    builder.add_iter_id(group_ids);
    builder.send(client)
        .context("failed to drop groups from role")?;

    Ok(())
}
