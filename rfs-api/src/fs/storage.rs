use std::path::PathBuf;
use std::collections::HashMap;

use rfs_lib::ids;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use snowcloud_flake::serde_ext::string_id;

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageLocal {
    pub path: PathBuf
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StorageType {
    Local(StorageLocal)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageListItem {
    #[serde(with = "string_id")]
    pub id: ids::StorageId,
    pub name: String,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: StorageType,
    pub tags: HashMap<String, Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateStorageType {
    Local {
        path: PathBuf
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStorage {
    pub name: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: CreateStorageType,
    pub tags: HashMap<String, Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageItem {
    #[serde(with = "string_id")]
    pub id: ids::StorageId,
    pub name: String,
    #[serde(with = "string_id")]
    pub user_id: ids::UserId,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: StorageType,
    pub tags: HashMap<String, Option<String>>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UpdateStorageType {
    Local {}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStorage {
    pub name: Option<String>,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: Option<UpdateStorageType>,
    pub tags: Option<HashMap<String, Option<String>>>,
}

impl UpdateStorage {
    pub fn has_work(&self) -> bool {
        self.name.is_some() ||
            self.type_.is_some() ||
            self.tags.is_some()
    }
}
