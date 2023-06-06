use std::path::PathBuf;

use super::ids::{StorageId, UserId, FSId};

pub enum StorageType {
    Local {
        directory: PathBuf,
    },
}

pub struct Storage {
    pub id: StorageId,
    pub name: String,
    pub user_id: UserId,
    pub type_: StorageType,
    pub root_fs: FSId,
    pub tags: Vec<String>,
    pub created: String,
    pub updated: Option<String>,
    pub deleted: Option<String>,
}
