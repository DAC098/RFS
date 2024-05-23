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
use rfs_api::users::ListItem;

use clap::{Subcommand, Args};

use crate::error::{self, Context};
use crate::util;
use crate::formatting::{Column, Float, PRETTY_OPTIONS, print_table};

mod group;

#[derive(Debug, Args)]
pub struct UsersArgs {
    #[command(subcommand)]
    command: UsersCmds
}

#[derive(Debug, Subcommand)]
enum UsersCmds {
    /// retrieves a list of users
    Get(GetArgs),

    /// creats a new user
    Create(CreateArgs),

    /// updates a user
    Update(UpdateArgs),

    /// interacts with users for a group
    Groups(group::GroupsArgs),
}

pub fn handle(client: &ApiClient, args: UsersArgs) -> error::Result {
    match args.command {
        UsersCmds::Get(given) => get(client, given),
        UsersCmds::Create(given) => create(client, given),
        UsersCmds::Update(given) => update(client, given),
        UsersCmds::Groups(given) => group::handle(client, given),
    }
}

fn insert_user<T>(list: &mut Vec<(ListItem, T)>, original: ListItem, output: T) {
    let pos = list.partition_point(|(v, _)| {
        match v.username.cmp(&original.username) {
            Ordering::Equal => v.id.id() < original.id.id(),
            Ordering::Less => true,
            Ordering::Greater => false,
        }
    });

    list.insert(pos, (original, output));
}

#[derive(Debug, Args)]
struct GetArgs {
    /// retrieves information about a single user
    #[arg(long, value_parser(util::parse_flake_id::<ids::UserId>))]
    id: Option<ids::UserId>,
}

fn get(client: &ApiClient, args: GetArgs) -> error::Result {
    if let Some(id) = args.id {
        let user = RetrieveUser::id(id)
            .send(client)
            .context("failed to retrieve user")?
            .context("user not found")?
            .into_payload();

        println!("{} {}", user.id.id(), user.username);

        if let Some(email) = user.email {
            println!(
                "email: {} {}",
                email.email,
                if email.verified { "verified" } else { "unverified" }
            );
        }
    } else {
        let mut builder = QueryUsers::new();
        let mut output_list: Vec<(ListItem, [Option<String>; 2])> = Vec::new();
        let mut columns = [
            Column::builder("id").float(Float::Right).build(),
            Column::builder("username").build(),
        ];

        for result in iterate::Iterate::new(client, &mut builder) {
            let user = result.context("failed to retrieve users")?;

            let mut output_user = std::array::from_fn(|_| None);

            output_user[0] = Some(user.id.id().to_string());
            output_user[1] = Some(user.username.clone());

            for (value, col) in output_user.iter().zip(&mut columns) {
                if let Some(st) = &value {
                    let chars_count = st.chars().count();

                    col.update_width(chars_count);
                }
            }

            insert_user(&mut output_list, user, output_user);
        }

        if output_list.is_empty() {
            println!("no contents");
        } else {
            print_table(&output_list, &columns, &PRETTY_OPTIONS)
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
    let mut builder = CreateUser::username(args.username);

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
    #[arg(long, value_parser(util::parse_flake_id::<rfs_lib::ids::UserId>))]
    id: rfs_lib::ids::UserId,

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
    let mut builder = UpdateUser::id(args.id);

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
