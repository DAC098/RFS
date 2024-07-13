use rfs_api::client::ApiClient;
use rfs_api::client::users::totp::{
    RetrieveTotp,
    CreateTotp,
    DeleteTotp,
    UpdateTotp,
};
use rfs_api::users::totp::Algo;

use clap::{Subcommand, Args, ValueEnum};

use crate::error::{self, Context};

mod recovery;

#[derive(Debug, Clone, ValueEnum)]
pub enum ValidAlgo {
    SHA1,
    SHA256,
    SHA512
}

impl From<ValidAlgo> for Algo {
    fn from(v: ValidAlgo) -> Algo {
        match v {
            ValidAlgo::SHA1 => Algo::SHA1,
            ValidAlgo::SHA256 => Algo::SHA256,
            ValidAlgo::SHA512 => Algo::SHA512,
        }
    }
}

#[derive(Debug, Args)]
pub struct TotpArgs {
    #[command(subcommand)]
    command: Option<TotpCmds>
}

#[derive(Debug, Subcommand)]
enum TotpCmds {
    /// enables totp 2FA
    Enable(EnableArgs),

    /// disabled totp 2FA
    Disable,

    /// updates totp data and regenerates the shared secret
    Update(UpdateArgs),

    /// interacts with totp recovery data
    Recovery(recovery::RecoveryArgs),
}

pub fn handle(client: &ApiClient, args: TotpArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            TotpCmds::Enable(given) => enable(client, given),
            TotpCmds::Disable => disable(client),
            TotpCmds::Update(given) => update(client, given),
            TotpCmds::Recovery(given) => recovery::handle(client, given),
        }
    } else {
        get(client)
    }
}

fn get(client: &ApiClient) -> error::Result {
    let result = RetrieveTotp::new()
        .send(client)
        .context("failed to retrieve totp data")?;

    if let Some(payload) = result {
        println!("{:?}", payload.into_payload());
    } else {
        println!("no totp 2FA enabled");
    }

    Ok(())
}

#[derive(Debug, Args)]
struct EnableArgs {
    /// specifies the algorithm to use for code generation
    #[arg(long)]
    algo: Option<ValidAlgo>,

    /// specifies the total digits required when entering codes
    #[arg(long)]
    digits: Option<u32>,

    /// specifies the amount of the time between generating codes
    #[arg(long)]
    step: Option<u64>,
}

fn enable(client: &ApiClient, args: EnableArgs) -> error::Result {
    let mut builder = CreateTotp::new();

    if let Some(algo) = args.algo {
        builder.algo(algo.into());
    }

    if let Some(digits) = args.digits {
        builder.digits(digits);
    }

    if let Some(step) = args.step {
        builder.step(step);
    }

    let result = builder.send(client)
        .context("failed to enable totp 2FA")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

fn disable(client: &ApiClient) -> error::Result {
    DeleteTotp::new()
        .send(client)
        .context("failed to disable totp 2FA")?;

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// specifies the algorithm to use for code generation
    #[arg(long)]
    algo: Option<ValidAlgo>,

    /// specifies the total digits required when entering codes
    #[arg(long)]
    digits: Option<u32>,

    /// specifies the amount of the time between generating codes
    #[arg(long)]
    step: Option<u64>,

    /// force regenerate the shared secret
    #[arg(long)]
    regen: bool
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result {
    let mut builder = UpdateTotp::new();

    if let Some(algo) = args.algo {
        builder.algo(algo.into());
    }

    if let Some(digits) = args.digits {
        builder.digits(digits);
    }

    if let Some(step) = args.step {
        builder.step(step);
    }

    builder.regen(args.regen);

    let result = builder.send(client)
        .context("failed to update totp")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}
