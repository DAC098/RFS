use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::tags;

use super::{traits, backend};

#[derive(Debug)]
pub struct Root {
    pub id: ids::FSSet,
    pub user: ids::UserSet,
    pub storage: ids::StorageSet,
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
            uid: root.id.into_uid(),
            user_uid: root.user.into_uid(),
            storage_uid: root.storage.into_uid(),
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
        self.id.local()
    }

    fn parent(&self) -> Option<&ids::FSId> {
        None
    }

    fn user_id(&self) -> &ids::UserId {
        self.user.local()
    }

    fn storage_id(&self) -> &ids::StorageId {
        self.storage.local()
    }

    fn full_path(&self) -> String {
        self.basename.clone()
    }

    fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    fn updated(&self) -> Option<&DateTime<Utc>> {
        self.updated.as_ref()
    }
}

impl traits::Container for Root {}
