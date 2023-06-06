use std::path::{PathBuf, Path};
use std::fmt::Write;

use futures::TryStream;
use deadpool_postgres::GenericClient;
use serde::{Serialize, Deserialize};
use lib::ids;

use crate::net;
use crate::storage;
use crate::tags;
use crate::util;

pub mod error;
pub mod checksum;
pub mod stream;

pub mod directory;
pub use directory::Directory;
pub mod file;
pub use file::File;

pub const FILE_TYPE: i16 = 1;
pub const DIR_TYPE: i16 = 2;

pub enum IdOption<'a> {
    Parent(&'a ids::FSId),
    Storage(&'a ids::StorageId)
}

pub async fn name_check<N>(
    conn: &impl GenericClient,
    id: &IdOption<'_>,
    name: N
) -> Result<Option<ids::FSId>, tokio_postgres::Error>
where
    N: AsRef<str>
{
    let check = match id {
        IdOption::Parent(parent) => conn.query_opt(
            "select id from fs where parent = $1 and basename = $2",
            &[parent, &name.as_ref()]
        ).await?,
        IdOption::Storage(storage_id) => conn.query_opt(
            "select id from fs where storage_id = $1 and basename = $2",
            &[storage_id, &name.as_ref()]
        ).await?
    };

    Ok(check.map(|row| row.get(0)))
}

pub async fn name_gen(
    conn: &impl GenericClient,
    id: &IdOption<'_>,
    mut attempts: usize
) -> Result<Option<String>, tokio_postgres::Error> {
    let now = util::utc_now().expect("failed to get utc now");
    let mut count = 1;
    let mut name = format!("{}_{}", now, count);

    while attempts != 0 {
        if name_check(conn, id, &name).await?.is_none() {
            return Ok(Some(name));
        }

        count += 1;

        name.clear();
        write!(&mut name, "{}_{}", now, count).unwrap();

        attempts -= 1;
    }

    Ok(None)
}

pub fn validate_dir<P>(name: &str, cwd: &PathBuf, path: P) -> std::io::Result<PathBuf>
where
    P: AsRef<Path>
{
    use std::io::{Error as IoError, ErrorKind};

    let path_ref = path.as_ref();

    let rtn = if !path_ref.is_absolute() {
        match std::fs::canonicalize(cwd.join(path_ref)) {
            Ok(p) => p,
            Err(err) => {
                return Err(match err.kind() {
                    ErrorKind::NotFound => {
                        let mut msg = String::new();
                        msg.push_str("given ");
                        msg.push_str(name);
                        msg.push_str(" does not exist");

                        IoError::new(ErrorKind::NotFound, msg)
                    },
                    _ => err
                });
            }
        }
    } else {
        path_ref.to_path_buf()
    };

    if !rtn.try_exists()? {
        let mut msg = String::new();
        msg.push_str("given ");
        msg.push_str(name);
        msg.push_str(" does not exist");

        return Err(IoError::new(ErrorKind::NotFound, msg));
    } else if !rtn.is_dir() {
        let mut msg = String::new();
        msg.push_str("given ");
        msg.push_str(name);
        msg.push_str(" is not a directory");

        return Err(IoError::new(ErrorKind::NotFound, msg));
    }

    Ok(rtn)
}
