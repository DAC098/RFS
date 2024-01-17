use std::path::PathBuf;

use rfs_api::client::ApiClient;
use rfs_api::client::fs::storage::{
    CreateStorage,
    RetrieveStorage,
    UpdateStorage,
};

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

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

pub fn create(client: &ApiClient, args: &ArgMatches) -> error::Result<()> {
    let name = args.get_one::<String>("name")
        .unwrap();
    let tags = util::tags_from_args("tag", args)?;

    match args.subcommand() {
        Some(("local", local_args)) => {
            let path = local_args.get_one::<PathBuf>("path").unwrap();

            let mut builder = CreateStorage::local(name, path);
            builder.add_iter_tags(tags);

            let result = builder.send(client)
                .context("failed to create storage")?
                .into_payload();

            println!("{:#?}", result);
        },
        _ => unreachable!()
    };

    Ok(())
}

pub fn update(client: &ApiClient, args: &ArgMatches) -> error::Result<()> {
    let id: rfs_lib::ids::StorageId = args.get_one::<i64>("id")
        .unwrap()
        .try_into()
        .context("invalid storage id format")?;

    let tags = util::tags_from_args("tag", args)?;
    let add_tags = util::tags_from_args("add-tag", args)?;
    let drop_tags = if let Some(given) = args.get_many::<String>("drop-tag") {
        given.collect()
    } else {
        Vec::new()
    };

    let mut current = {
        let result = RetrieveStorage::id(id.clone())
            .send(client)
            .context("failed to retrieve storage")?;

        let Some(payload) = result else {
            println!("storage not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateStorage::local(id);

    if let Some(name) = args.get_one::<String>("rename") {
        builder.name(name);
    }

    if !tags.is_empty() {
        builder.add_iter_tags(tags);
    } else if !drop_tags.is_empty() || !add_tags.is_empty() {
        for tag in drop_tags {
            current.tags.remove(tag);
        }

        for (tag, value) in add_tags {
            current.tags.insert(tag, value);
        }

        builder.add_iter_tags(current.tags);
    }

    let result = builder.send(client)
        .context("failed to update storage")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}
