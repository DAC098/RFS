use clap::ArgMatches;
use lib::ids;
use snowcloud_core::traits::FromIdGenerator;
use snowcloud_cloud::Generator;

use crate::error;

macro_rules! gen_id {
    ($fn_name:ident, $k:path, $name:expr) => {
        fn $fn_name() -> error::Result<()> {
            let mut generator: Generator<$k> = Generator::new(ids::START_TIME, 1)?;

            let id = generator.next_id()?;

            println!("{} id {}", $name,  id.id());

            Ok(())
        }
    }
}

gen_id!(gen_user_id, ids::UserId, "user");

pub fn run(args: &ArgMatches) -> error::Result<()> {
    match args.subcommand() {
        Some(("user", _)) => gen_user_id()?,
        _ => {}
    };

    Ok(())
}
