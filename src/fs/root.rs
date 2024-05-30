use std::path::PathBuf;

use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::tags;

use super::{traits, backend};

#[derive(Debug)]
pub struct Root {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage_id: ids::StorageId,
    pub basename: String,
    pub backend: backend::Node,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl Root {
    pub fn into_schema(self) -> rfs_api::fs::Root {
        self.into()
    }
}

impl From<Root> for rfs_api::fs::Root {
    fn from(root: Root) -> Self {
        rfs_api::fs::Root {
            id: root.id,
            user_id: root.user_id,
            storage_id: root.storage_id,
            basename: root.basename,
            backend: root.backend.into(),
            tags: root.tags,
            comment: root.comment,
            created: root.created,
            updated: root.updated,
            deleted: root.deleted,
        }
    }
}

impl traits::Common for Root {
    fn id(&self) -> &ids::FSId {
        &self.id
    }

    fn parent(&self) -> Option<&ids::FSId> {
        None
    }

    fn user_id(&self) -> &ids::UserId {
        &self.user_id
    }

    fn full_path(&self) -> PathBuf {
        PathBuf::new()
    }

    fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    fn updated(&self) -> Option<&DateTime<Utc>> {
        self.updated.as_ref()
    }
}

impl traits::Container for Root {}
