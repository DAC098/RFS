use rfs_api::client::ApiClient;
use rfs_api::client::auth::password::UpdatePassword;

use clap::{Subcommand, Args};

use crate::error::{self, Context};

#[derive(Debug, Args)]
pub struct PasswordArgs {
    #[command(subcommand)]
    command: PasswordCmds
}

#[derive(Debug, Subcommand)]
enum PasswordCmds {
    /// updates the current password to a new one
    Update,
}

pub fn handle(client: &ApiClient, args: PasswordArgs) -> error::Result {
    match args.command {
        PasswordCmds::Update => update(client),
    }
}

fn update(client: &ApiClient) -> error::Result {
    let current_prompt = "current: ";
    let updated_prompt = "updated: ";
    let confirm_prompt = "confirm: ";

    let current = rpassword::prompt_password(&current_prompt)?;
    let updated = rpassword::prompt_password(&updated_prompt)?;
    let mut confirm;

    loop {
        confirm = rpassword::prompt_password(&confirm_prompt)?;

        if confirm != updated {
            println!("updated and confirm do not match");
        } else {
            break;
        }
    }

    let mut builder = UpdatePassword::update_password(updated, confirm);

    if !current.is_empty() {
        builder.current(current);
    }

    builder.send(client)
        .context("failed to update password")?;

    Ok(())
}
