use rfs_api::client::ApiClient;
use rfs_api::client::sec::secrets::{
    CreatePasswordSecret,
    QueryPasswordSecrets,
    RetrievePasswordSecret,
    DeletePasswordSecret
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::formatting::{TextTable, Column, Float, PRETTY_OPTIONS};

#[derive(Debug, Args)]
pub struct PasswordArgs {
    #[command(subcommand)]
    command: PasswordCmds
}

#[derive(Debug, Subcommand)]
enum PasswordCmds {
    /// retrieves a list of known password secrets
    Get(GetArgs),

    /// creates a new password secret
    Update,

    /// removes a password secret
    Remove(RemoveArgs),
}

pub fn handle(client: &ApiClient, args: PasswordArgs) -> error::Result {
    match args.command {
        PasswordCmds::Get(given) => get(client, given),
        PasswordCmds::Update => update(client),
        PasswordCmds::Remove(given) => remove(client, given),
    }
}

#[derive(Debug, Args)]
struct GetArgs {
    /// the secret version to retrieve
    #[arg(long)]
    version: Option<u64>
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    if let Some(version) = args.version {
        let result = RetrievePasswordSecret::version(version)
            .send(client)
            .context("failed to retrieve password secret")?;

        if let Some(payload) = result {
            println!("{:?}", payload.into_payload());
        } else {
            println!("password secret not found");
        }
    } else {
        let result = QueryPasswordSecrets::new()
            .send(client)
            .context("failed to retrieve password secrets")?
            .into_payload();
        let mut table = TextTable::with_columns([
            Column::builder("version").float(Float::Right).build(),
            Column::builder("created").float(Float::Right).build(),
        ]);

        for secret in result {
            let mut row = table.add_row();
            row.set_col(0, secret.version);
            row.set_col(1, secret.created);
            row.finish(secret);
        }

        if table.is_empty() {
            println!("no secrets");
        } else {
            table.print(&PRETTY_OPTIONS)
                .context("failed to output results to stdout")?;
        }
    }

    Ok(())
}

fn update(client: &ApiClient) -> error::Result {
    CreatePasswordSecret::new()
        .send(client)
        .context("failed to create password secret")?;

    Ok(())
}

#[derive(Debug, Args)]
struct RemoveArgs {
    /// version of the secret to remove
    #[arg(long)]
    version: u64
}

fn remove(client: &ApiClient, args: RemoveArgs) -> error::Result {
    DeletePasswordSecret::version(args.version)
        .send(client)
        .context("failed to remove password secret")?;

    Ok(())
}
