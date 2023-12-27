use std::path::PathBuf;

use rfs_lib::ids;

pub trait Common {
    fn id(&self) -> &ids::FSId;

    fn full_path(&self) -> PathBuf;
}

pub trait Container: Common {}
