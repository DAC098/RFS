use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use snowcloud_flake::serde_ext::{option_string_id, string_id};

use crate::ids;
use crate::models::tags::Tags;
use crate::serde::{mime_str, mime_opt_str};

#[derive(Serialize, Deserialize)]
pub enum Type {
    File,
    Directory,
}

#[derive(Serialize, Deserialize)]
pub enum Storage {
    Local {
        id: ids::StorageId,
    }
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub enum Checksum {
    Blake3(String),

    Sha2_256(String),
    Sha2_512(String),

    Sha3_256(String),
    Sha3_512(String),
}

#[derive(Serialize, Deserialize)]
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
    pub checksums: Vec<Checksum>,
    pub storage: Storage,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub enum Item {
    Root(Root),
    File(File),
    Directory(Directory),
}
