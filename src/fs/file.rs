use std::path::PathBuf;

use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::storage;
use crate::tags;

use super::traits;

#[derive(Debug)]
pub struct File {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: storage::fs::Storage,
    pub parent: ids::FSId,
    pub path: PathBuf,
    pub basename: String,
    pub mime: mime::Mime,
    pub size: u64,
    pub hash: blake3::Hash,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl File {
    pub fn into_schema(self) -> rfs_api::fs::File {
        rfs_api::fs::File {
            id: self.id,
            user_id: self.user_id,
            parent: self.parent,
            basename: self.basename,
            path: self.path,
            size: self.size,
            mime: self.mime,
            tags: self.tags,
            comment: self.comment,
            hash: self.hash.as_bytes().to_vec(),
            storage: self.storage.into_schema(),
            created: self.created,
            updated: self.updated,
            deleted: self.deleted,
        }
    }
}

impl traits::Common for File {
    fn id(&self) -> &ids::FSId {
        &self.id
    }

    fn parent(&self) -> Option<&ids::FSId> {
        Some(&self.parent)
    }

    fn user_id(&self) -> &ids::UserId {
        &self.user_id
    }

    fn full_path(&self) -> PathBuf {
        self.path.join(&self.basename)
    }

    fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    fn updated(&self) -> Option<&DateTime<Utc>> {
        self.updated.as_ref()
    }
}
