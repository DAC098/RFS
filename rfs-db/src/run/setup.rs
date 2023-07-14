use clap::ArgMatches;

use crate::error;
use crate::conn;

pub async fn run(args: &ArgMatches) -> error::Result<()> {
    let mut conn = conn::postgres(args).await?;
    let current_dir = std::env::current_dir()?;

    let setup_dir = current_dir.join("rfs-db/setup/postgres");
    let read_dir = std::fs::read_dir(&setup_dir)?;

    let mut failed = false;
    let transaction = conn.transaction().await?;

    'read_dir: for entry in read_dir {
        let entry = entry?;
        let path = entry.path();

        tracing::event!(
            tracing::Level::INFO,
            file = %path.display(),
            "loading file"
        );

        let file_sql = std::fs::read_to_string(&path)
            .map_err(|err| error::Error::new()
                .kind("std::io::Error")
                .message(format!("failed to read file. {}", path.display()))
                .source(err))?;

        for sql in file_sql.split(';') {
            let trim = sql.trim();

            if let Err(err) = transaction.execute(trim, &[]).await {
                failed = true;

                println!(
                    "error running query from {}. {}\n{}", 
                    path.strip_prefix(&current_dir).unwrap().display(),
                    err,
                    trim
                );

                break 'read_dir;
            }
        }
    }

    if args.get_flag("rollback") || failed {
        tracing::event!(
            tracing::Level::INFO,
            "rollback changes"
        );

        transaction.rollback().await?;
    } else {
        tracing::event!(
            tracing::Level::INFO,
            "commit changes"
        );

        transaction.commit().await?;
    }

    Ok(())
}
