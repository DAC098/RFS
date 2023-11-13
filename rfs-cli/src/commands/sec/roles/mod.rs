use std::collections::HashSet;

use rfs_lib::sec::authz::permission::{Scope, Ability};
use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error;
use crate::util;
use crate::state::AppState;

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

pub fn get(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let given_id: bool;
    let path = if let Some(id) = args.get_one::<rfs_lib::ids::RoleId>("id") {
        given_id = true;
        format!("/sec/roles/{}", id)
    } else {
        given_id = false;
        format!("/sec/roles")
    };

    let url = state.server.url.join(&path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedRoleLookup")
            .message("failed to lookup desired role information")
            .source(json));
    }

    if given_id {
        let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::sec::Role>>()?;

        println!("{:#?}", result);
    } else {
        let result = res.json::<rfs_lib::json::ListWrapper<Vec<rfs_lib::schema::sec::RoleListItem>>>()?;

        println!("{:#?}", result);
    }

    Ok(())
}

#[inline]
fn perm_tuple_to_role_permission<T>(iter: T) -> Vec<rfs_lib::actions::sec::RolePermission>
where
    T: IntoIterator<Item = (Scope, Ability)>
{
    iter.into_iter()
        .map(|(scope, ability)| rfs_lib::actions::sec::RolePermission {
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
            .kind("InvalidFormat")
            .message("provided permissions are an invalid format")
            .source(format!("{:?}", invalid)));
    }

    if invalid_scope.len() != 0 {
        return Err(error::Error::new()
            .kind("InvalidPermissionScope")
            .message("provided permissions that have an invalid scope")
            .source(format!("{:?}", invalid_scope)));
    }

    if invalid_ability.len() != 0 {
        return Err(error::Error::new()
            .kind("InvalidPermissionAbility")
            .message("provided permissions that have an invalid ability")
            .source(format!("{:?}", invalid_ability)));
    }

    Ok(check)
}

pub fn create(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let permissions = perm_tuple_to_role_permission(parse_permissions("permission", args)?);
    let action = rfs_lib::actions::sec::CreateRole {
        name: args.get_one("name").cloned().unwrap(),
        permissions
    };

    let url = state.server.url.join("/sec/roles")?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedCreatingRole")
            .message("failed to create the desired role")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::sec::Role>>()?;

    println!("{:#?}", result);

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id: rfs_lib::ids::RoleId = args.get_one("id").cloned().unwrap();
    let path = format!("/sec/roles/{}", id);

    let permissions = parse_permissions("permission", args)?;
    let add_permissions = parse_permissions("add-permission", args)?;
    let drop_permissions = parse_permissions("drop-permission", args)?;

    let current = {
        let url = state.server.url.join(&path)?;
        let res = state.client.get(url).send()?;

        let status = res.status();

        if status != reqwest::StatusCode::OK {
            let json = res.json::<rfs_lib::json::Error>()?;

            return Err(error::Error::new()
                .kind("FailedRoleLookup")
                .message("failed to get the desired role")
                .source(json));
        }

        let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::sec::Role>>()?;

        result.into_payload()
    };

    let new_permissions = if permissions.len() > 0 {
        Some(perm_tuple_to_role_permission(permissions))
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

        Some(perm_tuple_to_role_permission(current_permissions))
    } else {
        None
    };

    let action = rfs_lib::actions::sec::UpdateRole {
        name: args.get_one("name").cloned(),
        permissions: new_permissions,
    };

    let url = state.server.url.join(&path)?;
    let res = state.client.patch(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedRoleUpdate")
            .message("failed to update the desired role")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::sec::Role>>()?;

    println!("{:#?}", result);

    Ok(())
}

pub fn delete(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let path = format!("/sec/roles/{}", id);

    let url = state.server.url.join(&path)?;
    let res = state.client.delete(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedRoleDelete")
            .message("failed to delete the desired role")
            .source(json));
    }

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

pub fn groups(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_matches)) => get_groups(state, get_matches),
        Some(("add", add_matches)) => add_groups(state, add_matches),
        Some(("drop", drop_matches)) => drop_groups(state, drop_matches),
        _ => unreachable!()
    }
}

pub fn get_users(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();

    let path = format!("/sec/roles/{}/users", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedRoleUsersLookup")
            .message("failed to retrieve a list of users in the desired role")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::ListWrapper<Vec<rfs_lib::schema::sec::RoleUser>>>()?;

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
                .message("A provided user id is not a valid format")
                .source(e))?;

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
                .kind("InvalidGroupId")
                .message("a provided group id is not a valid format"));
        }

        rtn.push(*id);
    }

    Ok(rtn)
}

pub fn add_users(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let action = rfs_lib::actions::sec::AddRoleUser {
        ids: get_user_ids(args)?
    };

    let path = format!("/sec/roles/{}/users", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedAddingRoleUsers")
            .message("failed to add new users to the desired role")
            .source(json));
    }

    Ok(())
}

pub fn drop_users(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let action = rfs_lib::actions::sec::DropRoleUser {
        ids: get_user_ids(args)?
    };

    let path = format!("/sec/roles/{}/users", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.delete(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedDroppingRoleUsers")
            .message("failed to drop users from the desired role")
            .source(json));
    }

    Ok(())
}

pub fn get_groups(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();

    let path = format!("/sec/roles/{}/groups", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedRoleGroupsLookup")
            .message("failed to retrieve a list of groups in the desired role")
            .source(json));
    }

    let result = res.json::<rfs_lib::json::ListWrapper<Vec<rfs_lib::schema::sec::RoleGroup>>>()?;

    println!("{:#?}", result);

    Ok(())
}

pub fn add_groups(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let action = rfs_lib::actions::sec::AddRoleGroup {
        ids: get_group_ids(args)?
    };

    let path = format!("/sec/roles/{}/groups", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedAddingRoleGroups")
            .message("failed to add new groups to the desired role")
            .source(json));
    }

    Ok(())
}

pub fn drop_groups(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let id = args.get_one::<rfs_lib::ids::RoleId>("id").unwrap();
    let action = rfs_lib::actions::sec::DropRoleGroup {
        ids: get_group_ids(args)?
    };

    let path = format!("/sec/roles/{}/groups", id);
    let url = state.server.url.join(&path)?;
    let res = state.client.delete(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedDroppingRoleGroups")
            .message("failed to drop groups from the desired role")
            .source(json));
    }

    Ok(())
}
