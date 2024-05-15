use std::path::PathBuf;

use rfs_lib::ids;
use rfs_lib::serde::{mime_str, mime_opt_str};

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use snowcloud_flake::serde_ext::string_id;

use crate::Tags;

pub mod storage;

#[derive(Debug, Serialize, Deserialize)]
pub enum Type {
    Root,
    File,
    Directory,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Storage {
    Local {
        id: ids::StorageId,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Root {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: Storage,
    pub tags: Tags,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RootMin {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(with = "string_id")]
    pub parent: ids::FSId,
    pub basename: String,
    pub path: PathBuf,
    pub size: u64,
    #[serde(with = "mime_str")]
    pub mime: mime::Mime,
    pub tags: Tags,
    pub comment: Option<String>,
    pub hash: Vec<u8>,
    pub storage: Storage,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMin {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(with = "string_id")]
    pub parent: ids::FSId,
    pub basename: String,
    pub path: PathBuf,
    pub size: u64,
    #[serde(with = "mime_str")]
    pub mime: mime::Mime,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMetadata {
    pub tags: Option<Tags>,
    pub comment: Option<String>,
}

impl UpdateMetadata {
    pub fn has_work(&self) -> bool {
        self.tags.is_some() ||
            self.comment.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateItem {
    Dir(CreateDir)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDir {
    pub basename: String,
    pub tags: Option<Tags>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItem {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(with = "string_id")]
    pub parent: ids::FSId,
    pub basename: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: Type,
    pub path: PathBuf,
    pub size: u64,
    #[serde(with = "mime_opt_str")]
    pub mime: Option<mime::Mime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Directory {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(with = "string_id")]
    pub parent: ids::FSId,
    pub basename: String,
    pub path: PathBuf,
    pub tags: Tags,
    pub comment: Option<String>,
    pub storage: Storage,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryMin {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(with = "string_id")]
    pub parent: ids::FSId,
    pub basename: String,
    pub path: PathBuf,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Item {
    Root(Root),
    File(File),
    Directory(Directory),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ItemMin {
    Root(RootMin),
    File(FileMin),
    Directory(DirectoryMin),
}
