use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::tags;

use super::{traits, backend};

#[derive(Debug)]
pub struct File {
    pub id: ids::FSSet,
    pub user: ids::UserSet,
    pub storage: ids::StorageSet,
    pub parent: ids::FSSet,
    pub path: String,
    pub basename: String,
    pub mime: mime::Mime,
    pub size: u64,
    pub hash: blake3::Hash,
    pub backend: backend::Node,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl File {
    pub fn into_schema(self) -> rfs_api::fs::File {
        self.into()
    }
}

impl From<File> for rfs_api::fs::File {
    fn from(file: File) -> Self {
        rfs_api::fs::File {
            uid: file.id.into_uid(),
            user_uid: file.user.into_uid(),
            storage_uid: file.storage.into_uid(),
            parent: file.parent.into_uid(),
            basename: file.basename,
            path: file.path,
            size: file.size,
            mime: file.mime,
            tags: file.tags,
            comment: file.comment,
            hash: file.hash.as_bytes().to_vec(),
            backend: file.backend.into(),
            created: file.created,
            updated: file.updated,
            deleted: file.deleted,
        }
    }
}

impl traits::Common for File {
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
