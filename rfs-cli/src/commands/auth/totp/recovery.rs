use clap::{Command, Arg, ArgAction, ArgMatches};

use crate::error;
use crate::util;
use crate::state::AppState;

pub fn command() -> Command {
    Command::new("recovery")
        .subcommand_required(true)
        .about("interacts with totp recovery data")
        .arg(util::default_help_arg())
        .subcommand(Command::new("get")
            .about("retrieves all recovery codes")
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("create")
            .about("creates a new recovery code")
            .arg(Arg::new("key")
                .long("key")
                .required(true)
                .help("key for the new recovery code")
            )
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("update")
            .about("updates a recovery code")
            .arg(Arg::new("key")
                .long("key")
                .required(true)
                .help("the desired key to update")
            )
            .arg(Arg::new("rename")
                .long("rename")
                .help("renames the specified key to the provided name")
            )
            .arg(Arg::new("regen")
                .long("regen")
                .action(ArgAction::SetTrue)
                .help("regenerates the specified key and resets its used status")
            )
            .arg(util::default_help_arg())
        )
        .subcommand(Command::new("delete")
            .about("deletes a recovery code")
            .arg(Arg::new("key")
                .long("key")
                .required(true)
                .help("the desired key to delete")
            )
            .arg(util::default_help_arg())
        )
}

fn print_recovery(recovery: &rfs_api::auth::totp::TotpRecovery) {
    print!("{} ", recovery.key);

    if recovery.used {
        print!("used\n");
    }

    println!("{}", recovery.hash);
}


pub fn get(state: &mut AppState, _args: &ArgMatches) -> error::Result {
    let path = "/auth/totp/recovery";
    let url = state.server.url.join(path)?;
    let res = state.client.get(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_api::error::ApiError>()?;

        return Err(error::Error::new()
            .kind("FailedTotpRecoveryLookup")
            .message("failed to retrieve totp recovery data")
            .source(json));
    }

    let result = res.json::<rfs_api::Payload<Vec<rfs_api::auth::totp::TotpRecovery>>>()?;

    for recovery in result.payload() {
        print_recovery(recovery);
    }

    Ok(())
}

pub fn create(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let action = rfs_api::auth::totp::CreateTotpHash {
        key: args.get_one("key").cloned().unwrap()
    };

    let path = "/auth/totp/recovery";
    let url = state.server.url.join(path)?;
    let res = state.client.post(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::CREATED {
        let json = res.json::<rfs_api::error::ApiError>()?;

        return Err(error::Error::new()
            .kind("FailedCreateTotpRecovery")
            .message("failed to create the totp recovery key")
            .source(json));
    }

    let result = res.json::<rfs_api::Payload<rfs_api::auth::totp::TotpRecovery>>()?;

    print_recovery(result.payload());

    Ok(())
}

pub fn update(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let key = args.get_one::<String>("key").cloned().unwrap();
    let path = format!("/auth/totp/recovery/{}", key);

    let action = rfs_api::auth::totp::UpdateTotpHash {
        key: args.get_one("rename").cloned(),
        regen: args.get_flag("regen")
    };

    let url = state.server.url.join(&path)?;
    let res = state.client.patch(url)
        .json(&action)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_api::error::ApiError>()?;

        return Err(error::Error::new()
            .kind("FailedUpdateTotpRecovery")
            .message("failed to update the totp recovery key")
            .source(json));
    }

    let result = res.json::<rfs_api::Payload<rfs_api::auth::totp::TotpRecovery>>()?;

    print_recovery(result.payload());

    Ok(())
}

pub fn delete(state: &mut AppState, args: &ArgMatches) -> error::Result {
    let key = args.get_one::<String>("key").cloned().unwrap();
    let path = format!("/auth/totp/recovery/{}", key);
    let url = state.server.url.join(&path)?;
    let res = state.client.delete(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_api::error::ApiError>()?;

        return Err(error::Error::new()
            .kind("FailedDeleteTotpRecovery")
            .message("failed to delete the totp recovery key")
            .source(json));
    }

    let _result = res.json::<rfs_api::Payload<()>>()?;

    Ok(())
}
