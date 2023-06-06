use std::path::PathBuf;

use super::ids::{FSId, StorageId, UserId};

pub enum FSType {
    File,
    Dir,
}

pub enum FSStorageData {
    Local {},
}

pub struct FS {
    pub id: FSId,
    pub type_: FSType,
    pub parent: Option<FSId>,
    pub users_id: UserId,
    pub root: bool,
    pub path: PathBuf,
    pub size: u64,
    pub mime: mime::Mime,
    pub tags: Vec<String>,
    pub storage_id: StorageId,
    pub storage_data: FSStorageData,
    pub created: String,
    pub updated: Option<String>,
    pub deleted: Option<String>,
}

