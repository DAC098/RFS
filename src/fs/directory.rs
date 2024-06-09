use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::tags;

use super::{traits, backend};

#[derive(Debug)]
pub struct Directory {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage_id: ids::StorageId,
    pub backend: backend::Node,
    pub parent: ids::FSId,
    pub basename: String,
    pub path: String,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl Directory {
    pub fn into_schema(self) -> rfs_api::fs::Directory {
        self.into()
    }
}

impl From<Directory> for rfs_api::fs::Directory {
    fn from(dir: Directory) -> Self {
        rfs_api::fs::Directory {
            id: dir.id,
            user_id: dir.user_id,
            storage_id: dir.storage_id,
            backend: dir.backend.into(),
            parent: dir.parent,
            basename: dir.basename,
            path: dir.path,
            tags: dir.tags,
            comment: dir.comment,
            created: dir.created,
            updated: dir.updated,
            deleted: dir.deleted,
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

    fn storage_id(&self) -> &ids::FSId {
        &self.storage_id
    }

    fn full_path(&self) -> String {
        format!("{}/{}", self.path, self.basename)
    }

    fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    fn updated(&self) -> Option<&DateTime<Utc>> {
        self.updated.as_ref()
    }
}

impl traits::Container for Directory {}
