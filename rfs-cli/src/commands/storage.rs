use std::path::PathBuf;

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error;
use crate::util;
use crate::state::AppState;

pub fn command() -> Command {
    Command::new("storage")
        .subcommand_required(true)
        .about("interacts with storage mediums on a server")
        .arg(util::default_help_arg())
        .subcommand(Command::new("create")
            .subcommand_required(true)
            .about("creates a new storage medium")
            .arg(util::default_help_arg())
            .arg(Arg::new("name")
                .short('n')
                .long("name")
                .required(true)
                .help("name of the new storage to create")
            )
            .arg(Arg::new("tag")
                .short('t')
                .long("tag")
                .help("tags to apply to the storage medium")
            )
            .arg(Arg::new("comment")
                .long("comment")
                .help("comment to apply to the storage medium")
            )
            .subcommand(Command::new("local")
                .about("creates a new storage medium that is local to the server")
                .arg(util::default_help_arg())
                .arg(Arg::new("path")
                    .short('p')
                    .long("path")
                    .value_parser(value_parser!(PathBuf))
                    .required(true)
                    .help("path on the server to create the local storage medium")
                )
            )
        )
        .subcommand(Command::new("update")
            .about("updates an existing")
            .arg(util::default_help_arg())
            .arg(Arg::new("id")
                .long("id")
                .value_parser(value_parser!(i64))
                .required(true)
                .help("the given id of the storage medium")
            )
            .arg(Arg::new("tag")
                .short('t')
                .long("tag")
                .action(ArgAction::Append)
                .help("tags to apply to the storage medium. overrides currently set tags")
                .conflicts_with_all(["add-tag"])
            )
            .arg(Arg::new("add-tag")
                .long("add-tag")
                .action(ArgAction::Append)
                .help("adds a tag to the given storage medium")
                .conflicts_with("tag")
            )
            .arg(Arg::new("drop-tag")
                .long("drop-tag")
                .action(ArgAction::Append)
                .help("drops a tag from the given storage medium")
                .conflicts_with("tag")
            )
            .arg(Arg::new("rename")
                .long("rename")
                .help("renames the given storage medium")
            )
            .arg(Arg::new("comment")
                .long("comment")
                .help("updates the comment of the given storage medium")
            )
        )
}

pub fn create(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let name = args.get_one::<String>("name")
        .cloned()
        .unwrap();
    let tags = util::tags_from_args("tag", args)?;

    let type_ = match args.subcommand() {
        Some(("local", local_args)) => {
            let path: PathBuf = local_args.get_one::<PathBuf>("path")
                .cloned()
                .unwrap();

            rfs_lib::actions::storage::CreateStorageType::Local {
                path
            }
        },
        _ => unreachable!()
    };

    let action = rfs_lib::actions::storage::CreateStorage {
        name, type_, tags
    };

    tracing::event!(
        tracing::Level::DEBUG,
        "action: {:#?}",
        action
    );

    let res = state.client.post(state.server.url.join("/storage")?)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedCreatingStorage")
            .message("failed to create the new storage medium")
            .source(format!("{:?}", json)));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::storage::StorageItem>>()?;

    println!("{:?}", result.into_payload());

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    let id = args.get_one::<i64>("id").unwrap();
    let path = format!("/storage/{}", id);

    let tags = util::tags_from_args("tag", args)?;
    let add_tags = util::tags_from_args("add-tag", args)?;
    let drop_tags = if let Some(given) = args.get_many::<String>("drop-tag") {
        given.collect()
    } else {
        Vec::new()
    };

    let mut current = {
        let res = state.client.get(state.server.url.join(&path)?)
            .send()?;

        let status = res.status();

        if status != reqwest::StatusCode::OK {
            let json = res.json::<rfs_lib::json::Error>()?;

            return Err(error::Error::new()
                .kind("FailedStorageLookup")
                .message("failed to get the desired storage medium")
                .source(format!("{:?}", json)));
        }

        let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::storage::StorageItem>>()?;

        tracing::event!(
            tracing::Level::INFO,
            "current storage medium: {:?}",
            result
        );

        result.into_payload()
    };

    let type_action = match &current.type_ {
        rfs_lib::schema::storage::StorageType::Local(_) => {
            None
        }
    };

    let new_tags = {
        if tags.len() > 0 {
            Some(tags)
        } else if drop_tags.len() > 0 || add_tags.len() > 0 {
            for tag in drop_tags {
                current.tags.remove(tag);
            }

            for (tag, value) in add_tags {
                current.tags.insert(tag, value);
            }

            Some(current.tags)
        } else {
            None
        }
    };

    let action = rfs_lib::actions::storage::UpdateStorage {
        name: args.get_one::<String>("rename").cloned(),
        type_: type_action,
        tags: new_tags
    };

    if !action.has_work() {
        println!("no changes have been specified");
        return Ok(());
    }

    let res = state.client.put(state.server.url.join(&path)?)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedUpdateStorage")
            .message("failed to update the desired storage medium")
            .source(format!("{:?}", json)));
    }

    let result = res.json::<rfs_lib::json::Wrapper<rfs_lib::schema::storage::StorageItem>>()?;

    tracing::event!(
        tracing::Level::INFO,
        "{:?}",
        result
    );

    Ok(())
}
