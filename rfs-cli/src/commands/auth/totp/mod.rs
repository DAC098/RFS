use rfs_api::client::ApiClient;
use rfs_api::client::auth::totp::{
    RetrieveTotp,
    CreateTotp,
    DeleteTotp,
    UpdateTotp,
};
use rfs_api::auth::totp::Algo;

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error::{self, Context};
use crate::util;

mod recovery;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ValidAlgo {
    SHA1,
    SHA256,
    SHA512
}

impl ValidAlgo {
    pub fn as_string(&self) -> String {
        match self {
            ValidAlgo::SHA1 => String::from("SHA1"),
            ValidAlgo::SHA256 => String::from("SHA256"),
            ValidAlgo::SHA512 => String::from("SHA512")
        }
    }
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

impl From<&ValidAlgo> for Algo {
    fn from(v: &ValidAlgo) -> Algo {
        match v {
            ValidAlgo::SHA1 => Algo::SHA1,
            ValidAlgo::SHA256 => Algo::SHA256,
            ValidAlgo::SHA512 => Algo::SHA512,
        }
    }
}

pub fn command() -> Command {
    Command::new("totp")
        .subcommand_required(true)
        .about("interacts with data specific to totp")
        .arg(util::default_help_arg())
        .subcommand(Command::new("get")
            .about("retrieves any currently available totp data")
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("enable")
            .about("enables totp 2FA for the current user")
            .arg(Arg::new("algo")
                .long("algo")
                .value_parser(value_parser!(ValidAlgo))
                .help("specifies the algorithm to use for code generation")
            )
            .arg(Arg::new("digits")
                .long("digits")
                .value_parser(value_parser!(u32))
                .help("specifies the total digits required when entering codes")
            )
            .arg(Arg::new("step")
                .long("step")
                .value_parser(value_parser!(u64))
                .help("specifies the amount of time between generating codes")
            )
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("disable")
            .about("disables totp 2FA for the current user")
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("update")
            .about("updates current totp data and regenerates the shared secret")
            .arg(Arg::new("algo")
                .long("algo")
                .value_parser(value_parser!(ValidAlgo))
                .help("specifies the algorithm to use for code generation")
            )
            .arg(Arg::new("digits")
                .long("digits")
                .value_parser(value_parser!(u32))
                .help("specifies the total digits required when entering codes")
            )
            .arg(Arg::new("step")
                .long("step")
                .value_parser(value_parser!(u64))
                .help("specifies the amount of time between generating codes")
            )
            .arg(Arg::new("regen")
                .long("regen")
                .action(ArgAction::SetTrue)
                .help("force regenerates the shared secret")
            )
            .arg(util::default_help_arg())
        )
        .subcommand(recovery::command())
}

pub fn get(client: &ApiClient) -> error::Result {
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

pub fn enable(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let mut builder = CreateTotp::new();

    if let Some(algo) = args.get_one::<ValidAlgo>("algo") {
        builder.algo(algo.into());
    }

    if let Some(digits) = args.get_one::<u32>("digits") {
        builder.digits(*digits);
    }

    if let Some(step) = args.get_one::<u64>("step") {
        builder.step(*step);
    }

    let result = builder.send(client)
        .context("failed to enable totp 2FA")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

pub fn disable(client: &ApiClient) -> error::Result {
    DeleteTotp::new()
        .send(client)
        .context("failed to disable totp 2FA")?;

    Ok(())
}

pub fn update(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let mut builder = UpdateTotp::new();

    if let Some(algo) = args.get_one::<ValidAlgo>("algo") {
        builder.algo(algo.into());
    }

    if let Some(digits) = args.get_one::<u32>("digits") {
        builder.digits(*digits);
    }

    if let Some(step) = args.get_one::<u64>("step") {
        builder.step(*step);
    }

    if args.get_flag("regen") {
        builder.regen(true);
    }

    let result = builder.send(client)
        .context("failed to update totp")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

pub fn recovery(client: &ApiClient, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", _)) => recovery::get(client),
        Some(("create", create_args)) => recovery::create(client, create_args),
        Some(("update", update_args)) => recovery::update(client, update_args),
        Some(("delete", delete_args)) => recovery::delete(client, delete_args),
        _ => unreachable!()
    }
}

