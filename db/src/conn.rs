use std::io::Write;
use std::str::FromStr;

use clap::ArgMatches;
use tokio_postgres::{Client, Config, NoTls};

use crate::error;

pub async fn postgres(args: &ArgMatches) -> error::Result<Client> {
    let (client, conn) = if let Some(connect) = args.get_one::<String>("connect") {
        Config::from_str(connect.as_str())?
            .connect(NoTls)
            .await?
    } else {
        let user = args.get_one::<String>("user")
            .unwrap();
        let password = args.get_one::<String>("password");
        let host = args.get_one::<String>("host")
            .unwrap();
        let port = args.get_one("port")
            .map(|v: &u16| v.clone())
            .unwrap();
        let dbname = args.get_one::<String>("dbname")
            .unwrap();

        let mut config = Config::new();
        config.user(user.as_str());
        config.host(host.as_str());
        config.port(port);
        config.dbname(dbname.as_str());

        if args.get_flag("req_password") {
            let prompt = format!("{} password: ", user);
            let input = rpassword::prompt_password(prompt)?;

            if let Some((given, _newline)) = input.rsplit_once('\n') {
                config.password(given);
            } else {
                config.password(input.as_str());
            }
        } else if let Some(pass) = password {
            config.password(pass.as_str());
        }

        config.connect(NoTls).await?
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::event!(
                tracing::Level::ERROR,
                "postgres connection error: {}",
                e
            );
        }
    });

    Ok(client)
}
