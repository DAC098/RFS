use std::cmp::Ordering;

use rfs_lib::ids;
use rfs_api::client::ApiClient;
use rfs_api::client::iterate;
use rfs_api::client::users::{
    QueryUsers,
    RetrieveUser,
    CreateUser,
    UpdateUser,
};
use rfs_api::client::users::groups::QueryGroupUsers;
use rfs_api::users::ListItem;

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::formatting::{TextTable, Column, Float, PRETTY_OPTIONS};

mod group;

#[derive(Debug, Args)]
pub struct UsersArgs {
    #[command(flatten)]
    get: GetArgs,

    #[command(subcommand)]
    command: Option<UsersCmds>
}

#[derive(Debug, Subcommand)]
enum UsersCmds {
    /// creats a new user
    Create(CreateArgs),

    /// updates a user
    Update(UpdateArgs),

    /// interacts with users for a group
    Groups(group::GroupsArgs),
}

pub fn handle(client: &ApiClient, args: UsersArgs) -> error::Result {
    if let Some(cmd) = args.command {
        match cmd {
            UsersCmds::Create(given) => create(client, given),
            UsersCmds::Update(given) => update(client, given),
            UsersCmds::Groups(given) => group::handle(client, given),
        }
    } else {
        get(client, args.get)
    }
}

fn sort_user(a: &ListItem, b: &ListItem) -> bool {
    match b.username.cmp(&a.username) {
        Ordering::Equal => b.uid < a.uid,
        Ordering::Less => true,
        Ordering::Greater => false,
    }
}

#[derive(Debug, Args)]
struct GetArgs {
    /// retrieves information about a single user
    #[arg(
        long,
        conflicts_with("group")
    )]
    uid: Option<ids::UserUid>,

    /// retrieves users that are in a specific group
    #[arg(long, conflicts_with("id"))]
    group: Option<ids::GroupUid>,
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    if let Some(uid) = args.uid {
        let user = RetrieveUser::uid(uid)
            .send(client)
            .context("failed to retrieve user")?
            .context("user not found")?
            .into_payload();

        println!("{} {}", user.uid, user.username);

        if let Some(email) = user.email {
            println!(
                "email: {} {}",
                email.email,
                if email.verified { "verified" } else { "unverified" }
            );
        }
    } else if let Some(group) = args.group {
        let mut builder = QueryGroupUsers::uid(group);
        let mut table = TextTable::with_columns([
            Column::builder("uid").float(Float::Right).build(),
        ]);

        for result in iterate::Iterate::new(client, &mut builder) {
            let user = result.context("failed to retrieve group users")?;
            let mut row = table.add_row();
            row.set_col(0, user.uid.clone());

            row.finish(user);
        }

        if table.is_empty() {
            println!("no contents");
        } else {
            table.print(&PRETTY_OPTIONS)
                .context("failed to output results to stdout")?;
        }
    } else {
        let mut builder = QueryUsers::new();
        let mut table = TextTable::with_columns([
            Column::builder("uid").float(Float::Right).build(),
            Column::builder("username").build(),
        ]);

        for result in iterate::Iterate::new(client, &mut builder) {
            let user = result.context("failed to retrieve users")?;
            let mut row = table.add_row();
            row.set_col(0, user.uid.clone());
            row.set_col(1, user.username.clone());

            row.finish_sort_by(user, sort_user);
        }

        if table.is_empty() {
            println!("no contents");
        } else {
            table.print(&PRETTY_OPTIONS)
                .context("failed to output results to stdout")?;
        }
    }

    Ok(())
}

#[derive(Debug, Args)]
struct CreateArgs {
    /// username of new user
    #[arg(long)]
    username: String,

    /// email of new user
    #[arg(long)]
    email: Option<String>
}

fn create(client: &ApiClient, args: CreateArgs) -> error::Result<()> {
    let password = rpassword::prompt_password("new password: ")?;

    loop {
        let confirm = rpassword::prompt_password("confirm password: ")?;

        if confirm != password {
            println!("confirm password does not match");
        } else {
            break;
        }
    }

    let mut builder = CreateUser::username(args.username, password);

    if let Some(email) = args.email {
        builder.email(email);
    }

    let result = builder.send(client)
        .context("failed to create new user")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// id of the user to update
    #[arg(long)]
    uid: ids::UserUid,

    /// updates username
    #[arg(long)]
    username: Option<String>,

    /// updates email
    #[arg(long, conflicts_with("no_email"))]
    email: Option<String>,

    /// removes the email
    #[arg(long, conflicts_with("email"))]
    no_email: bool
}

fn update(client: &ApiClient, args: UpdateArgs) -> error::Result<()> {
    let mut builder = UpdateUser::uid(args.uid);

    if let Some(username) = args.username {
        builder.username(username);
    }

    if let Some(given) = args.email {
        builder.email(Some(given));
    } else if args.no_email {
        builder.email(None::<String>);
    }

    let result = builder.send(client)
        .context("failed to update desired user")?
        .into_payload();

    println!("{:?}", result);

    Ok(())
}
