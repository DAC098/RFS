use std::path::PathBuf;

use rfs_lib::ids;
use chrono::{DateTime, Utc};

use crate::tags;

use super::{traits, backend};

#[derive(Debug)]
pub struct File {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage_id: ids::StorageId,
    pub parent: ids::FSId,
    pub path: PathBuf,
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
            id: file.id,
            user_id: file.user_id,
            storage_id: file.storage_id,
            parent: file.parent,
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
