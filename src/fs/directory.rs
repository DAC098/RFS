use std::path::PathBuf;

use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::storage;
use crate::tags;

use super::traits;

#[derive(Debug)]
pub struct Directory {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: storage::fs::Storage,
    pub parent: ids::FSId,
    pub basename: String,
    pub path: PathBuf,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: chrono::DateTime<chrono::Utc>,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

impl Directory {
    pub fn into_schema(self) -> rfs_api::fs::Directory {
        rfs_api::fs::Directory {
            id: self.id,
            user_id: self.user_id,
            storage: self.storage.into_schema(),
            parent: self.parent,
            basename: self.basename,
            path: self.path,
            tags: self.tags,
            comment: self.comment,
            created: self.created,
            updated: self.updated,
            deleted: self.deleted,
        }
    }
}

impl traits::Common for Directory {
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

impl traits::Container for Directory {}
