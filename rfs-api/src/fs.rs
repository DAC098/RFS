use std::path::PathBuf;

use rfs_lib::ids;
use rfs_lib::serde::mime_str;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use snowcloud_flake::serde_ext::string_id;

use crate::Tags;

pub mod backend;

#[derive(Debug, Serialize, Deserialize)]
pub struct Root {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(with = "string_id")]
    pub storage_id: ids::StorageId,
    pub basename: String,
    pub backend: backend::Node,
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
    #[serde(with = "string_id")]
    pub storage_id: ids::StorageId,
    pub basename: String,
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
    pub storage_id: ids::StorageId,
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
    pub backend: backend::Node,
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
    pub storage_id: ids::StorageId,
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
pub struct Directory {
    #[serde(with = "string_id")]
    pub id: ids::FSId,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(with = "string_id")]
    pub storage_id: ids::StorageId,
    #[serde(with = "string_id")]
    pub parent: ids::FSId,
    pub basename: String,
    pub path: PathBuf,
    pub tags: Tags,
    pub comment: Option<String>,
    pub backend: backend::Node,
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
    pub storage_id: ids::StorageId,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStorage {
    pub name: String,
    pub backend: backend::CreateConfig,
    pub tags: Tags,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Storage {
    #[serde(with = "string_id")]
    pub id: ids::StorageId,
    pub name: String,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    pub backend: backend::Config,
    pub tags: Tags,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageMin {
    #[serde(with = "string_id")]
    pub id: ids::StorageId,
    pub name: String,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    pub backend: backend::Config,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStorage {
    pub name: Option<String>,
    pub backend: Option<backend::UpdateConfig>,
    pub tags: Option<Tags>,
}

impl UpdateStorage {
    pub fn has_work(&self) -> bool {
        self.name.is_some() ||
            self.backend.is_some() ||
            self.tags.is_some()
    }
}
