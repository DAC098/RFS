use rfs_api::client::ApiClient;
use rfs_api::client::auth::totp::{
    RetrieveTotpRecovery,
    CreateTotpRecovery,
    UpdateTotpRecovery,
    DeleteTotpRecovery,
};

use clap::{Subcommand, Args};

use crate::error::{self, Context};

#[derive(Debug, Args)]
pub struct RecoveryArgs {
    #[command(subcommand)]
    command: RecoveryCmds
}

#[derive(Debug, Subcommand)]
enum RecoveryCmds {
    /// retrieves all recovery codes
    Get,

    /// creates a new recovery code
    Create(CreateArgs),

    /// updates a recovery code
    Update(UpdateArgs),

    /// deletes a recovery code
    Delete(DeleteArgs)
}

pub fn handle(client: &ApiClient, args: RecoveryArgs) -> error::Result {
    match args.command {
        RecoveryCmds::Get => get(client),
        RecoveryCmds::Create(given) => create(client, given),
        RecoveryCmds::Update(given) => update(client, given),
        RecoveryCmds::Delete(given) => delete(client, given),
    }
}

fn print_recovery(recovery: &rfs_api::auth::totp::TotpRecovery) {
    print!("{} ", recovery.key);

    if recovery.used {
        print!("used\n");
    }

    println!("{}", recovery.hash);
}


fn get(client: &ApiClient) -> error::Result {
    let result = RetrieveTotpRecovery::new()
        .send(client)
        .context("failed to retrieve totp recovery keys")?
        .into_payload();

    for recovery in result {
        print_recovery(&recovery);
    }

    Ok(())
}

#[derive(Debug, Args)]
struct CreateArgs {
    /// key for the new recovery code
    #[arg(long)]
    key: String,
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result {
    let result = CreateTotpRecovery::key(args.key)
        .send(client)
        .context("failed to create totp recovery key")?
        .into_payload();

    print_recovery(&result);

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// the desired key to update
    #[arg(long)]
    key: String,

    /// renamed the recovery key to the one provided
    #[arg(long)]
    rename: Option<String>,

    /// regenerates the recovery key and resets its used status
    #[arg(long)]
    regen: bool
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result {
    let mut builder = UpdateTotpRecovery::key(args.key);

    if let Some(rename) = args.rename {
        builder.rename(rename);
    }

    builder.regen(args.regen);

    let result = builder.send(client)
        .context("failed to update totp recovery key")?
        .into_payload();

    print_recovery(&result);

    Ok(())
}

#[derive(Debug, Args)]
struct DeleteArgs {
    /// the desired key to delete
    #[arg(long)]
    key: String
}

fn delete(client: &ApiClient, args: DeleteArgs) -> error::Result {
    DeleteTotpRecovery::key(args.key)
        .send(client)
        .context("failed to delete totp recovery key")?;

    Ok(())
}
