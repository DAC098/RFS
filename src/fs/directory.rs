use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::tags;

use super::{traits, backend};

#[derive(Debug)]
pub struct Directory {
    pub id: ids::FSSet,
    pub user: ids::UserSet,
    pub storage: ids::StorageSet,
    pub backend: backend::Node,
    pub parent: ids::FSSet,
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
            uid: dir.id.into_uid(),
            user_uid: dir.user.into_uid(),
            storage_uid: dir.storage.into_uid(),
            backend: dir.backend.into(),
            parent: dir.parent.into_uid(),
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
        self.id.local()
    }

    fn parent(&self) -> Option<&ids::FSId> {
        Some(self.parent.local())
    }

    fn user_id(&self) -> &ids::UserId {
        self.user.local()
    }

    fn storage_id(&self) -> &ids::StorageId {
       self.storage.local()
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
