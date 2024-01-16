use rfs_api::auth::totp::Algo;

use clap::{Command, Arg, ArgAction, ArgMatches, value_parser};

use crate::error;
use crate::util;
use crate::state::AppState;

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

pub fn get(state: &mut AppState, _args: &ArgMatches) -> error::Result {
    let path = "/auth/totp";
    let url = state.server.url.join(path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        match status {
            reqwest::StatusCode::NOT_FOUND => {
                println!("totp is not enabled for this user");
            },
            _ => {
                let json = res.json::<rfs_api::error::ApiError>()?;

                return Err(error::Error::new()
                    .kind("FailedTotpLookup")
                    .message("failed to retrieve totp data")
                    .source(json));
            }
        }
    } else {
        let result = res.json::<rfs_api::Payload<rfs_api::auth::totp::Totp>>()?;

        println!("{:?}", result);
    }

    Ok(())
}

pub fn enable(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let action = rfs_api::auth::totp::CreateTotp {
        algo: args.get_one("algo").map(|v: &ValidAlgo| v.into()),
        digits: args.get_one("digits").cloned(),
        step: args.get_one("step").cloned(),
    };

    let path = "/auth/totp";
    let url = state.server.url.join(path)?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::CREATED {
        let json = res.json::<rfs_api::error::ApiError>()?;

        return Err(error::Error::new()
            .kind("FailedEnablingTotp")
            .message("failed to enable totp 2FA")
            .source(json));
    }

    let result = res.json::<rfs_api::Payload<rfs_api::auth::totp::Totp>>()?;

    println!("{:?}", result);

    Ok(())
}

pub fn disable(state: &mut AppState, _args: &ArgMatches) -> error::Result {
    let path = "/auth/totp";
    let url = state.server.url.join(path)?;
    let res = state.client.delete(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_api::error::ApiError>()?;

        return Err(error::Error::new()
            .kind("FailedDisablingTotp")
            .message("failed to disable totp 2FA")
            .source(json));
    }

    let result = res.json::<rfs_api::Payload<()>>()?;

    println!("{:?}", result);

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let action = rfs_api::auth::totp::UpdateTotp {
        algo: args.get_one("algo").map(|v: &ValidAlgo| v.into()),
        digits: args.get_one("digits").cloned(),
        step: args.get_one("step").cloned(),
        regen: args.get_flag("regen"),
    };

    let path = "/auth/totp";
    let url = state.server.url.join(path)?;
    let res = state.client.patch(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_api::error::ApiError>()?;

        return Err(error::Error::new()
            .kind("FailedUpdatingTotp")
            .message("failed to update totp")
            .source(json));
    }

    let result = res.json::<rfs_api::Payload<rfs_api::auth::totp::Totp>>()?;

    println!("{:?}", result);

    Ok(())
}

pub fn recovery(state: &mut AppState, args: &ArgMatches) -> error::Result {
    match args.subcommand() {
        Some(("get", get_args)) => recovery::get(state, get_args),
        Some(("create", create_args)) => recovery::create(state, create_args),
        Some(("update", update_args)) => recovery::update(state, update_args),
        Some(("delete", delete_args)) => recovery::delete(state, delete_args),
        _ => unreachable!()
    }
}

