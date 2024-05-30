use std::path::PathBuf;

use rfs_lib::ids;
use deadpool_postgres::GenericClient;
use tokio_postgres::Error as PgError;
use chrono::{DateTime, Utc};

use crate::tags;
use crate::sql;

pub mod consts;
pub mod traits;

pub mod root;
pub use root::Root;

pub mod directory;
pub use directory::Directory;

pub mod file;
pub use file::File;

pub mod backend;

pub async fn name_check<N>(
    conn: &impl GenericClient,
    parent: &ids::FSId,
    name: N
) -> Result<Option<ids::FSId>, tokio_postgres::Error>
where
    N: AsRef<str>
{
    let check = conn.query_opt(
        "select id from fs where parent = $1 and basename = $2",
        &[parent, &name.as_ref()]
    ).await?;

    Ok(check.map(|row| row.get(0)))
}

#[derive(Debug)]
pub enum Item {
    Root(Root),
    Directory(Directory),
    File(File),
}

impl Item {
    fn query_to_item(
        row: tokio_postgres::Row,
        tags: tags::TagMap
    ) -> Result<Item, PgError> {
        let fs_type = row.get(5);

        let item = match fs_type {
            consts::ROOT_TYPE => Item::Root(Root {
                id: row.get(0),
                user_id: row.get(1),
                storage_id: row.get(2),
                basename: row.get(4),
                backend: sql::de_from_sql(row.get(11)),
                tags,
                comment: row.get(12),
                created: row.get(13),
                updated: row.get(14),
                deleted: row.get(15)
            }),
            consts::FILE_TYPE => Item::File(File {
                id: row.get(0),
                user_id: row.get(1),
                storage_id: row.get(2),
                backend: sql::de_from_sql(row.get(11)),
                parent: row.get(3),
                path: sql::pathbuf_from_sql(row.get(6)),
                basename: row.get(4),
                mime: sql::mime_from_sql(row.get(8), row.get(9)),
                size: sql::u64_from_sql(row.get(7)),
                hash: sql::blake3_hash_from_sql(row.get(10)),
                tags,
                comment: row.get(12),
                created: row.get(13),
                updated: row.get(14),
                deleted: row.get(15),
            }),
            consts::DIR_TYPE => Item::Directory(Directory {
                id: row.get(0),
                user_id: row.get(1),
                storage_id: row.get(2),
                backend: sql::de_from_sql(row.get(11)),
                parent: row.get(3),
                path: sql::pathbuf_from_sql(row.get(6)),
                basename: row.get(4),
                tags,
                comment: row.get(12),
                created: row.get(13),
                updated: row.get(14),
                deleted: row.get(15),
            }),
            _ => {
                panic!("unexpected fs_type when retrieving fs Item. type: {}", fs_type);
            }
        };

        Ok(item)
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::FSId
    ) -> Result<Option<Item>, PgError> {
        let record_params: sql::ParamsVec = vec![id];

        let record_query = conn.query_opt(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.storage_id, \
                   fs.parent, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.hash, \
                   fs.backend, \
                   fs.comment, \
                   fs.created, \
                   fs.updated, \
                   fs.deleted \
            from fs \
            where fs.id = $1",
            record_params.as_slice()
        );
        let tags_query = tags::get_tags(conn, "fs_tags", "fs_id", id);

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags)) => Ok(Some(Self::query_to_item(row, tags)?)),
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn id(&self) -> &ids::FSId {
        match self {
            Self::Root(root) => &root.id,
            Self::Directory(dir) => &dir.id,
            Self::File(file) => &file.id,
        }
    }

    pub fn user_id(&self) -> &ids::UserId {
        match self {
            Self::Root(root) => &root.user_id,
            Self::Directory(dir) => &dir.user_id,
            Self::File(file) => &file.user_id,
        }
    }

    pub fn as_container(&self) -> Option<&dyn traits::Container> {
        match self {
            Self::Root(root) => Some(root),
            Self::Directory(dir) => Some(dir),
            Self::File(_file) => None,
        }
    }

    pub fn storage_id(&self) -> &ids::StorageId {
        match self {
            Self::Root(root) => &root.storage_id,
            Self::Directory(dir) => &dir.storage_id,
            Self::File(file) => &file.storage_id,
        }
    }

    pub fn set_comment(&mut self, comment: Option<String>) -> Option<String> {
        match self {
            Self::Root(root) => std::mem::replace(&mut root.comment, comment),
            Self::Directory(dir) => std::mem::replace(&mut dir.comment, comment),
            Self::File(file) => std::mem::replace(&mut file.comment, comment),
        }
    }

    pub fn set_tags(&mut self, tags: tags::TagMap) -> tags::TagMap {
        match self {
            Self::Root(root) => std::mem::replace(&mut root.tags, tags),
            Self::Directory(dir) => std::mem::replace(&mut dir.tags, tags),
            Self::File(file) => std::mem::replace(&mut file.tags, tags),
        }
    }

    pub fn try_into_file(self) -> Option<File> {
        match self {
            Self::File(file) => Some(file),
            _ => None
        }
    }

    pub fn into_file(self) -> File {
        self.try_into_file().expect("fs Item did not contain a file")
    }

    pub fn into_schema(self) -> rfs_api::fs::Item {
        self.into()
    }
}

impl traits::Common for Item {
    fn id(&self) -> &ids::FSId {
        Item::id(self)
    }

    fn parent(&self) -> Option<&ids::FSId> {
        match self {
            Self::Root(root) => root.parent(),
            Self::Directory(dir) => dir.parent(),
            Self::File(file) => file.parent(),
        }
    }

    fn user_id(&self) -> &ids::UserId {
        Item::user_id(self)
    }

    fn full_path(&self) -> PathBuf {
        match self {
            Self::Root(root) => root.full_path(),
            Self::Directory(dir) => dir.full_path(),
            Self::File(file) => file.full_path(),
        }
    }

    fn created(&self) -> &DateTime<Utc> {
        match self {
            Self::Root(root) => root.created(),
            Self::Directory(dir) => dir.created(),
            Self::File(file) => file.created(),
        }
    }

    fn updated(&self) -> Option<&DateTime<Utc>> {
        match self {
            Self::Root(root) => root.updated(),
            Self::Directory(dir) => dir.updated(),
            Self::File(file) => file.updated(),
        }
    }
}

impl From<Root> for Item {
    fn from(root: Root) -> Self {
        Item::Root(root)
    }
}

impl From<Directory> for Item {
    fn from(dir: Directory) -> Self {
        Item::Directory(dir)
    }
}

impl From<File> for Item {
    fn from(file: File) -> Self {
        Item::File(file)
    }
}

impl From<Item> for rfs_api::fs::Item {
    fn from(item: Item) -> Self {
        match item {
            Item::Root(root) => rfs_api::fs::Item::Root(root.into()),
            Item::Directory(dir) => rfs_api::fs::Item::Directory(dir.into()),
            Item::File(file) => rfs_api::fs::Item::File(file.into()),
        }
    }
}

pub struct Storage {
    pub id: ids::StorageId,
    pub name: String,
    pub user_id: ids::UserId,
    pub backend: backend::Config,
    pub tags: tags::TagMap,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl Storage {
    pub async fn name_check<N>(
        conn: &impl GenericClient,
        name: N
    ) -> Result<Option<ids::StorageId>, PgError>
    where
        N: AsRef<str>
    {
        if let Some(row) = conn.query_opt(
            "select id from storage where name = $1",
            &[&name.as_ref()]
        ).await? {
            Ok(row.get(0))
        } else {
            Ok(None)
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::StorageId,
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsVec = vec![id];

        let record_query = conn.query_opt(
            "\
            select storage.id, \
                   storage.user_id, \
                   storage.name, \
                   storage.backend, \
                   storage.created, \
                   storage.updated, \
                   storage.deleted \
            from storage \
            where storage.id = $1",
            record_params.as_slice()
        );
        let tags_query = tags::get_tags(
            conn,
            "storage_tags",
            "storage_id",
            id
        );

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags)) => {
                Ok(Some(Storage {
                    id: row.get(0),
                    name: row.get(2),
                    user_id: row.get(1),
                    backend: sql::de_from_sql(row.get(3)),
                    tags,
                    created: row.get(4),
                    updated: row.get(5),
                    deleted: row.get(6),
                }))
            },
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn into_schema(self) -> rfs_api::fs::Storage {
        self.into()
    }
}

impl From<Storage> for rfs_api::fs::Storage {
    fn from(storage: Storage) -> Self {
        rfs_api::fs::Storage {
            id: storage.id,
            name: storage.name,
            user_id: storage.user_id,
            backend: storage.backend.into(),
            tags: storage.tags,
            created: storage.created,
            updated: storage.updated,
            deleted: storage.deleted,
        }
    }
}
