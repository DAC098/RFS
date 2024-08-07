use std::path::PathBuf;

use rfs_api::client::{ApiClient, iterate};
use rfs_api::client::fs::storage::{
    QueryStorage,
    CreateStorage,
    RetrieveStorage,
    UpdateStorage,
};
use rfs_api::fs::{
    StorageMin,
    backend,
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::util;
use crate::formatting::{self, WriteTags, OutputOptions, TextTable, Column, Float, PRETTY_OPTIONS};

#[derive(Debug, Args)]
pub struct StorageArgs {
    #[command(flatten)]
    get: GetArgs,

    #[command(subcommand)]
    command: Option<StorageCmds>
}

#[derive(Debug, Subcommand)]
enum StorageCmds {
    /// creates a new storage medium
    Create(CreateArgs),
    /// updates an existing storage medium
    Update(UpdateArgs),
}

pub fn handle(client: &ApiClient, args: StorageArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            StorageCmds::Create(given) => create(client, given),
            StorageCmds::Update(given) => update(client, given),
        }
    } else {
        get(client, args.get)
    }
}

fn sort_storage(a: &StorageMin, b: &StorageMin) -> bool {
    a.name < b.name
}

#[derive(Debug, Args)]
struct GetArgs {
    /// uid of the storage item to retrieve
    #[arg(long)]
    uid: Option<rfs_lib::ids::StorageUid>,

    #[command(flatten)]
    output_options: OutputOptions,
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    if let Some(uid) = args.uid {
        let found = RetrieveStorage::uid(uid)
            .send(client)
            .context("failed to retrieve desired storage")?
            .context("storage id not found")?
            .into_payload();

        println!("{} {}", found.name, found.uid);
        println!("owner: {}", found.user_uid);
        println!("created: {}", formatting::datetime_to_string(&found.created, &args.output_options.ts_format));

        if let Some(updated) = found.updated {
            println!("updated: {}", formatting::datetime_to_string(&updated, &args.output_options.ts_format));
        }

        match found.backend {
            backend::Config::Local(local) => {
                println!("backend: Local");
                println!("    path: \"{}\"", local.path.display());
            }
        }

        println!("{}", WriteTags::new(&found.tags));
    } else {
        let mut builder = QueryStorage::new();
        let mut table = TextTable::with_columns([
            Column::builder("uid").float(Float::Right).build(),
            Column::builder("name").build(),
            Column::builder("type").build(),
            //Column::builder("mod").float(Float::Right).build(),
        ]);

        for result in iterate::Iterate::new(client, &mut builder) {
            let item = result.context("failed to retrieve storage item")?;
            let mut row = table.add_row();

            //let time = item.updated.as_ref().unwrap_or(&item.created);

            row.set_col(0, item.uid.clone());
            row.set_col(1, item.name.clone());
            //row.set_col(3, formatting::datetime_to_string(&time, &args.output_options.ts_format));

            match &item.backend {
                backend::Config::Local(_) => {
                    row.set_col(2, "Local");
                }
            }

            row.finish_sort_by(item, sort_storage);
        }

        if table.is_empty() {
            println!("no contents");
        } else {
            table.print(&PRETTY_OPTIONS)
                .context("failed to outpu results to stdout")?;
        }
    }

    Ok(())
}

#[derive(Debug, Args)]
struct CreateArgs {
    /// name of the new storage to create
    #[arg(short, long)]
    name: String,

    /// tags to apply
    #[arg(short, long = "tag", value_parser(util::parse_tag))]
    tags: Vec<util::Tag>,

    /// comment to apply
    #[arg(short, long)]
    comment: Option<String>,

    /// the type of item to create
    #[command(subcommand)]
    create_type: CreateType,
}

#[derive(Debug, Subcommand)]
enum CreateType {
    /// creates a storage medium that is local to the server.
    Local {
        /// path on the server to create the local storage
        #[arg(long)]
        path: PathBuf
    }
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result<()> {
    match args.create_type {
        CreateType::Local { path } => {
            let mut builder = CreateStorage::local(args.name, path);

            if let Some(comment) = args.comment {
                builder.comment(comment);
            }

            builder.add_iter_tags(args.tags);

            let result = builder.send(client)
                .context("failed to create storage")?
                .into_payload();

            println!("{:#?}", result);
        }
    }

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// uid of storage medium to update
    #[arg(long)]
    uid: rfs_lib::ids::StorageUid,

    #[command(flatten)]
    tags: Option<util::TagArgs>,

    /// updates the comment of the storage medium
    #[arg(long)]
    comment: Option<String>,

    /// renames the given storage medium
    #[arg(long)]
    rename: Option<String>,
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result<()> {
    let current = {
        let result = RetrieveStorage::uid(args.uid.clone())
            .send(client)
            .context("failed to retrieve storage")?;

        let Some(payload) = result else {
            println!("storage not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateStorage::local(args.uid);

    if let Some(rename) = args.rename {
        builder.name(rename);
    }

    if let Some(tags) = args.tags {
        builder.add_iter_tags(tags.merge_existing(current.tags));
    }

    let result = builder.send(client)
        .context("failed to update storage")?
        .into_payload();

    println!("{:#?}", result);

    Ok(())
}
