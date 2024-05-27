use std::path::PathBuf;

use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::storage;
use crate::tags;

use super::traits;

#[derive(Debug)]
pub struct Root {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: storage::fs::Storage,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl Root {
    pub fn into_schema(self) -> rfs_api::fs::Root {
        rfs_api::fs::Root {
            id: self.id,
            user_id: self.user_id,
            storage: self.storage.into_schema(),
            tags: self.tags,
            comment: self.comment,
            created: self.created,
            updated: self.updated,
            deleted: self.deleted,
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
