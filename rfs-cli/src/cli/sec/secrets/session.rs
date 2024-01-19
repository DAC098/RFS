use rfs_api::client::ApiClient;
use rfs_api::client::sec::secrets::{
    CreateSessionSecret,
    DeleteSessionSecret,
    QuerySessionSecrets,
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    command: SessionCmds
}

#[derive(Debug, Subcommand)]
enum SessionCmds {
    /// retrieves a list of known session secrets
    Get,

    /// creates a new session secret
    Update,

    /// removes the oldest session secret
    Remove
}

pub fn handle(client: &ApiClient, args: SessionArgs) -> error::Result {
    match args.command {
        SessionCmds::Get => get(client),
        SessionCmds::Update => update(client),
        SessionCmds::Remove => remove(client),
    }
}

fn get(client: &ApiClient) -> error::Result {
    let result = QuerySessionSecrets::new()
        .send(client)
        .context("failed to retrieve session secrets")?
        .into_payload();

    for secret in result {
        println!("{:?}", secret);
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
