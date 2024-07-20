use rfs_api::client::ApiClient;
use rfs_api::client::sec::secrets::{
    CreateSessionSecret,
    DeleteSessionSecret,
    QuerySessionSecrets,
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::formatting::{TextTable, Column, Float, PRETTY_OPTIONS};

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    command: Option<SessionCmds>,
}

#[derive(Debug, Subcommand)]
enum SessionCmds {
    /// creates a new session secret
    Update,

    /// removes the oldest session secret
    Remove
}

pub fn handle(client: &ApiClient, args: SessionArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            SessionCmds::Update => update(client),
            SessionCmds::Remove => remove(client),
        }
    } else {
        get(client)
    }
}

fn get(client: &ApiClient) -> error::Result {
    let result = QuerySessionSecrets::new()
        .send(client)
        .context("failed to retrieve session secrets")?
        .into_payload();
    let mut table = TextTable::with_columns([
        Column::builder("created").float(Float::Right).build()
    ]);

    for secret in result {
        let mut row = table.add_row();
        row.set_col(0, secret.created);
        row.finish(secret);
    }

    if table.is_empty() {
        println!("no secrets");
    } else {
        table.print(&PRETTY_OPTIONS)
            .context("failed to output to stdout")?;
    }

    Ok(())
}

fn update(client: &ApiClient) -> error::Result {
    CreateSessionSecret::new()
        .send(client)
        .context("failed to create session secret")?;

    Ok(())
}

fn remove(client: &ApiClient) -> error::Result {
    DeleteSessionSecret::amount(1)
        .send(client)
        .context("failed to remove session secret")?;

    Ok(())
}
