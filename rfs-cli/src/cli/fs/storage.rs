use std::path::PathBuf;

use rfs_api::client::ApiClient;
use rfs_api::client::fs::storage::{
    CreateStorage,
    RetrieveStorage,
    UpdateStorage,
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::util;

#[derive(Debug, Args)]
pub struct StorageArgs {
    #[command(subcommand)]
    command: StorageCmds
}

#[derive(Debug, Subcommand)]
enum StorageCmds {
    /// creates a new storage medium
    Create(CreateArgs),
    /// updates an existing storage medium
    Update(UpdateArgs),
}

pub fn handle(client: &ApiClient, args: StorageArgs) -> error::Result {
    match args.command {
        StorageCmds::Create(given) => create(client, given),
        StorageCmds::Update(given) => update(client, given),
    }
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
    /// id of storage medium to update
    #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::StorageId>))]
    id: rfs_lib::ids::StorageId,

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
        let result = RetrieveStorage::id(args.id.clone())
            .send(client)
            .context("failed to retrieve storage")?;

        let Some(payload) = result else {
            println!("storage not found");
            return Ok(());
        };

        payload.into_payload()
    };

    let mut builder = UpdateStorage::local(args.id);

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
