use clap::ArgMatches;

use crate::error;
use crate::conn;

pub async fn run(args: &ArgMatches) -> error::Result<()> {
    let mut conn = conn::postgres(args).await?;
    let current_dir = std::env::current_dir()?;

    let setup_dir = current_dir.join("db/setup/postgres");
    let read_dir = std::fs::read_dir(&setup_dir)?;

    let mut failed = false;
    let mut transaction = conn.transaction().await?;

    'read_dir: for entry in read_dir {
        let entry = entry?;
        let path = entry.path();

        let file_sql = std::fs::read_to_string(&path)?;

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
        transaction.rollback().await?;
    } else {
        transaction.commit().await?;
    }

    Ok(())
}
