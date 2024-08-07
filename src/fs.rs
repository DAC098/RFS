use rfs_lib::ids;
use deadpool_postgres::GenericClient;
use tokio_postgres::Error as PgError;
use chrono::{DateTime, Utc};

use crate::tags;
use crate::sql;

pub mod consts;
pub mod traits;

use traits::Common;

pub mod root;
pub use root::Root;

pub mod directory;
pub use directory::Directory;

pub mod file;
pub use file::File;

pub mod backend;

#[derive(Debug)]
pub enum Item {
    Root(Root),
    Directory(Directory),
    File(File),
}

impl Item {
    pub async fn name_check(
        conn: &impl GenericClient,
        parent: &ids::FSId,
        name: &str
    ) -> Result<Option<ids::FSId>, tokio_postgres::Error> {
        if let Some(row) = conn.query_opt(
            "select id from fs where parent = $1 and basename = $2",
            &[parent, &name]
        ).await? {
            Ok(Some(row.get(0)))
        } else {
            Ok(None)
        }
    }

    fn query_to_item(
        row: tokio_postgres::Row,
        tags: tags::TagMap
    ) -> Result<Item, PgError> {
        let fs_type = row.get(9);

        let id = ids::FSSet::new(row.get(0), row.get(1));
        let user = ids::UserSet::new(row.get(2), row.get(3));
        let storage = ids::StorageSet::new(row.get(4), row.get(5));
        let basename = row.get(8);
        let backend = sql::de_from_sql(row.get(15));
        let comment = row.get(16);
        let created = row.get(17);
        let updated = row.get(18);
        let deleted = row.get(19);

        let item = match fs_type {
            consts::ROOT_TYPE => Item::Root(Root {
                id,
                user,
                storage,
                basename,
                backend,
                tags,
                comment,
                created,
                updated,
                deleted,
            }),
            consts::FILE_TYPE => Item::File(File {
                id,
                user,
                storage,
                parent: ids::FSSet::new(row.get(6), row.get(7)),
                backend,
                path: row.get(10),
                basename,
                mime: sql::mime_from_sql(row.get(12), row.get(13)),
                size: sql::u64_from_sql(row.get(11)),
                hash: sql::blake3_hash_from_sql(row.get(14)),
                tags,
                comment,
                created,
                updated,
                deleted,
            }),
            consts::DIR_TYPE => Item::Directory(Directory {
                id,
                user,
                storage,
                backend,
                parent: ids::FSSet::new(row.get(6), row.get(7)),
                path: row.get(10),
                basename,
                tags,
                comment,
                created,
                updated,
                deleted,
            }),
            _ => {
                panic!("unexpected fs_type when retrieving fs Item. type: {}", fs_type);
            }
        };

        Ok(item)
    }

    fn retrieve_base_query() -> &'static str {
            "\
            select fs.id, \
                   fs.uid, \
                   users.id, \
                   users.uid, \
                   storage.id, \
                   storage.uid, \
                   fs_parent.id, \
                   fs_parent.uid, \
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
            left join users on \
                fs.user_id = users.id \
            left join storage on \
                fs.storage_id = storage.id \
            left join fs as fs_parent on \
                fs.parent = fs_parent.id"
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::FSId
    ) -> Result<Option<Item>, PgError> {
        let record_param: sql::ParamsArray<'_, 1> = [id];
        let record_query = format!("{} where fs.id = $1", Self::retrieve_base_query());

        let record_fut = conn.query_opt(&record_query, &record_param);
        let tags_fut = tags::get_tags(conn, "fs_tags", "fs_id", id);

        match tokio::try_join!(record_fut, tags_fut)? {
            (Some(row), tags) => Ok(Some(Self::query_to_item(row, tags)?)),
            (None, _) => Ok(None),
        }
    }

    pub async fn retrieve_uid(
        conn: &impl GenericClient,
        uid: &ids::FSUid
    ) -> Result<Option<Item>, PgError> {
        let record_param: sql::ParamsArray<'_, 1> = [uid];
        let tag_param: sql::ParamsArray<'_, 1> = [uid];
        let record_query = format!("{} where fs.uid = $1", Self::retrieve_base_query());

        tracing::debug!("retrieving record");

        let record = conn.query_opt(&record_query, &record_param).await?;

        tracing::debug!("retrieving tags");

        let tags = conn.query_raw(
            "\
            select fs_tags.tag, \
                   fs_tags.value \
            from fs_tags \
            left join fs on \
                fs_tags.fs_id = fs.id \
            where fs.uid = $1",
            tag_param
        ).await?;

        //match tokio::try_join!(record_fut, tags_fut)? {
        match (record, tags) {
            (Some(row), tags) => {
                let tags = tags::from_row_stream(tags).await?;

                Ok(Some(Self::query_to_item(row, tags)?))
            }
            (None, _) => Ok(None),
        }
    }

    pub fn id(&self) -> &ids::FSSet {
        match self {
            Self::Root(root) => &root.id,
            Self::Directory(dir) => &dir.id,
            Self::File(file) => &file.id,
        }
    }

    pub fn user(&self) -> &ids::UserSet {
        match self {
            Self::Root(root) => &root.user,
            Self::Directory(dir) => &dir.user,
            Self::File(file) => &file.user,
        }
    }

    pub fn as_container(&self) -> Option<&dyn traits::Container> {
        match self {
            Self::Root(root) => Some(root),
            Self::Directory(dir) => Some(dir),
            Self::File(_file) => None,
        }
    }

    pub fn storage(&self) -> &ids::StorageSet {
        match self {
            Self::Root(root) => &root.storage,
            Self::Directory(dir) => &dir.storage,
            Self::File(file) => &file.storage,
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

    pub fn try_into_parent_parts(self) -> Result<(ids::FSSet, String, backend::Node), Self> {
        match self {
            Self::Root(root) => {
                let full_path = root.full_path();

                Ok((root.id, full_path, root.backend))
            }
            Self::Directory(dir) => {
                let full_path = dir.full_path();

                Ok((dir.id, full_path, dir.backend))
            }
            Self::File(file) => Err(Item::File(file))
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

impl Common for Item {
    fn id(&self) -> &ids::FSId {
        match self {
            Self::Root(root) => root.id.local(),
            Self::Directory(dir) => dir.id.local(),
            Self::File(file) => file.id.local(),
        }
    }

    fn parent(&self) -> Option<&ids::FSId> {
        match self {
            Self::Root(root) => root.parent(),
            Self::Directory(dir) => dir.parent(),
            Self::File(file) => file.parent(),
        }
    }

    fn user_id(&self) -> &ids::UserId {
        match self {
            Self::Root(root) => root.user.local(),
            Self::Directory(dir) => dir.user.local(),
            Self::File(file) => file.user.local(),
        }
    }

    fn storage_id(&self) -> &ids::StorageId {
        match self {
            Self::Root(root) => root.storage.local(),
            Self::Directory(dir) => dir.storage.local(),
            Self::File(file) => file.storage.local(),
        }
    }

    fn full_path(&self) -> String {
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

impl TryFrom<Item> for File {
    type Error = Item;

    fn try_from(item: Item) -> Result<Self, Self::Error> {
        match item {
            Item::File(file) => Ok(file),
            _ => Err(item)
        }
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
    pub id: ids::StorageSet,
    pub name: String,
    pub user: ids::UserSet,
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

    fn retrieve_base_query() -> &'static str {
        "\
        select storage.id, \
               storage.uid, \
               users.id, \
               users.uid, \
               storage.name, \
               storage.backend, \
               storage.created, \
               storage.updated, \
               storage.deleted \
        from storage \
            join users on storage.user_id = users.id"
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::StorageId,
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsArray<1> = [id];
        let record_query = format!("{} where storage.id = $1", Self::retrieve_base_query());

        let record_fut = conn.query_opt(&record_query, &record_params);
        let tags_fut = tags::get_tags(conn, "storage_tags", "storage_id", id);

        match tokio::try_join!(record_fut, tags_fut)? {
            (Some(row), tags) => {
                Ok(Some(Storage {
                    id: ids::StorageSet::new(row.get(0), row.get(1)),
                    user: ids::UserSet::new(row.get(2), row.get(3)),
                    name: row.get(4),
                    backend: sql::de_from_sql(row.get(5)),
                    tags,
                    created: row.get(6),
                    updated: row.get(7),
                    deleted: row.get(8),
                }))
            },
            (None, _) => Ok(None),
        }
    }

    pub async fn retrieve_uid(
        conn: &impl GenericClient,
        uid: &ids::StorageUid,
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsArray<1> = [uid];
        let tags_params: sql::ParamsArray<1> = [uid];
        let record_query = format!("{} where storage.uid = $1", Self::retrieve_base_query());

        let record_fut = conn.query_opt(&record_query, &record_params);
        let tags_fut = conn.query_raw(
            "\
            select storage_tags.tag, \
                   storage_tags.value \
            from storage_tags \
                join storage on storage_tags.storage_id = storage.id \
            where storage.uid = $1",
            tags_params
        );

        match tokio::try_join!(record_fut, tags_fut)? {
            (Some(row), tags) => {
                Ok(Some(Storage {
                    id: ids::StorageSet::new(row.get(0), row.get(1)),
                    user: ids::UserSet::new(row.get(2), row.get(3)),
                    name: row.get(4),
                    backend: sql::de_from_sql(row.get(5)),
                    tags: tags::from_row_stream(tags).await?,
                    created: row.get(6),
                    updated: row.get(7),
                    deleted: row.get(8),
                }))
            }
            (None, _) => Ok(None),
        }
    }

    fn from_fs_base_query() -> &'static str {
            "\
            select storage.id, \
                   storage.uid, \
                   users.id, \
                   users.uid, \
                   storage.name, \
                   storage.backend, \
                   storage.created, \
                   storage.updated, \
                   storage.deleted \
            from storage \
                join fs on storage.id = fs.storage_id \
                join users on storage.user_id = users.id"
    }

    pub async fn from_fs_id(
        conn: &impl GenericClient,
        fs_id: &ids::FSId
    ) -> Result<Option<Self>, PgError> {
        let record_param: sql::ParamsArray<'_, 1> = [fs_id];
        let tags_param: sql::ParamsArray<'_, 1> = [fs_id];
        let record_query = format!("{} where fs.id = $1", Self::from_fs_base_query());

        let record_fut = conn.query_opt(&record_query, &record_param);
        let tags_fut = conn.query_raw(
            "\
            select storage_tags.tag, \
                   storage_tags.value \
            from storage_tags \
                join fs on storage_tags.storage_id = fs.storage_id \
            where fs.id = $1",
            tags_param
        );

        match tokio::try_join!(record_fut, tags_fut)? {
            (Some(row), tags) => {
                Ok(Some(Storage {
                    id: ids::StorageSet::new(row.get(0), row.get(1)),
                    user: ids::UserSet::new(row.get(2), row.get(3)),
                    name: row.get(4),
                    backend: sql::de_from_sql(row.get(5)),
                    tags: tags::from_row_stream(tags).await?,
                    created: row.get(6),
                    updated: row.get(7),
                    deleted: row.get(8),
                }))
            },
            (None, _) => Ok(None),
        }
    }

    pub async fn from_fs_uid(
        conn: &impl GenericClient,
        fs_uid: &ids::FSUid,
    ) -> Result<Option<Self>, PgError> {
        let record_param: sql::ParamsArray<'_, 1> = [fs_uid];
        let tags_param: sql::ParamsArray<'_, 1> = [fs_uid];
        let record_query = format!("{} where fs.uid = $1", Self::from_fs_base_query());
        let record_fut = conn.query_opt(&record_query, &record_param);
        let tags_fut = conn.query_raw(
            "\
            select storage_tags.tag, \
                   storage_tags.value \
            from storage_tags \
                join fs on storage_tags.storage_id = fs.storage_id \
            where fs.uid = $1",
            tags_param
        );

        match tokio::try_join!(record_fut, tags_fut)? {
            (Some(row), tags) => {
                Ok(Some(Storage {
                    id: ids::StorageSet::new(row.get(0), row.get(1)),
                    user: ids::UserSet::new(row.get(2), row.get(3)),
                    name: row.get(4),
                    backend: sql::de_from_sql(row.get(5)),
                    tags: tags::from_row_stream(tags).await?,
                    created: row.get(6),
                    updated: row.get(7),
                    deleted: row.get(8),
                }))
            }
            (None, _) => Ok(None)
        }
    }

    pub fn into_schema(self) -> rfs_api::fs::Storage {
        self.into()
    }
}

impl From<Storage> for rfs_api::fs::Storage {
    fn from(storage: Storage) -> Self {
        rfs_api::fs::Storage {
            uid: storage.id.into_uid(),
            name: storage.name,
            user_uid: storage.user.into_uid(),
            backend: storage.backend.into(),
            tags: storage.tags,
            created: storage.created,
            updated: storage.updated,
            deleted: storage.deleted,
        }
    }
}

// ----------------------------------------------------------------------------

use crate::error::{ApiError, ApiResult};
use crate::error::api::{ApiErrorKind, Context};
use crate::sec::authn::initiator::Initiator;

pub async fn fetch_item_uid(
    conn: &impl GenericClient,
    uid: &ids::FSUid,
    initiator: &Initiator,
) -> ApiResult<Item> {
    let item = Item::retrieve_uid(conn, uid)
        .await
        .context("failed to retrieve fs item by uid")?
        .kind(ApiErrorKind::FileNotFound)?;

    if initiator.user.id != *item.user_id() {
        Err(ApiError::from(ApiErrorKind::PermissionDenied))
    } else {
        Ok(item)
    }
}

pub async fn fetch_storage_from_fs_uid(
    conn: &impl GenericClient,
    fs_uid: &ids::FSUid,
) -> ApiResult<Storage> {
    Storage::from_fs_uid(conn, fs_uid)
        .await
        .context("failed to retrieve storage item from fs uid")?
        .kind(ApiErrorKind::StorageNotFound)
}
