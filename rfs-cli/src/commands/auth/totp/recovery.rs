use rfs_api::client::ApiClient;
use rfs_api::client::auth::totp::{
    RetrieveTotpRecovery,
    CreateTotpRecovery,
    UpdateTotpRecovery,
    DeleteTotpRecovery,
};

use clap::{Command, Arg, ArgAction, ArgMatches};

use crate::error::{self, Context};
use crate::util;

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


pub fn get(client: &ApiClient) -> error::Result {
    let result = RetrieveTotpRecovery::new()
        .send(client)
        .context("failed to retrieve totp recovery keys")?
        .into_payload();

    for recovery in result {
        print_recovery(&recovery);
    }

    Ok(())
}

pub fn create(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let key = args.get_one::<String>("key").cloned().unwrap();
    let result = CreateTotpRecovery::key(key)
        .send(client)
        .context("failed to create totp recovery key")?
        .into_payload();

    print_recovery(&result);

    Ok(())
}

pub fn update(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let key = args.get_one::<String>("key").cloned().unwrap();
    let mut builder = UpdateTotpRecovery::key(key);

    if let Some(rename) = args.get_one::<String>("rename") {
        builder.rename(rename);
    }

    if args.get_flag("regen") {
        builder.regen(true);
    }

    let result = builder.send(client)
        .context("failed to update totp recovery key")?
        .into_payload();

    print_recovery(&result);

    Ok(())
}

pub fn delete(client: &ApiClient, args: &ArgMatches) -> error::Result {
    let key = args.get_one::<String>("key").cloned().unwrap();

    DeleteTotpRecovery::key(key)
        .send(client)
        .context("failed to delete totp recovery key")?;

    Ok(())
}
