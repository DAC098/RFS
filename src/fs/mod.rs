use std::path::PathBuf;

use rfs_lib::ids;
use rfs_lib::schema;
use deadpool_postgres::GenericClient;
use tokio_postgres::Error as PgError;

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
        let fs_type = row.get(4);

        let item = match fs_type {
            consts::ROOT_TYPE => {
                Item::Root(Root {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(10)),
                    tags,
                    comment: row.get(11),
                    created: row.get(12),
                    updated: row.get(13),
                    deleted: row.get(14)
                })
            },
            consts::FILE_TYPE => {
                Item::File(File {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(10)),
                    parent: row.get(2),
                    path: sql::pathbuf_from_sql(row.get(5)),
                    basename: row.get(3),
                    mime: sql::mime_from_sql(row.get(7), row.get(8)),
                    size: sql::u64_from_sql(row.get(6)),
                    hash: sql::blake3_hash_from_sql(row.get(9)),
                    tags,
                    comment: row.get(11),
                    created: row.get(12),
                    updated: row.get(13),
                    deleted: row.get(14),
                })
            },
            consts::DIR_TYPE => {
                Item::Directory(Directory {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(10)),
                    parent: row.get(2),
                    path: sql::pathbuf_from_sql(row.get(5)),
                    basename: row.get(3),
                    tags,
                    comment: row.get(11),
                    created: row.get(12),
                    updated: row.get(13),
                    deleted: row.get(14),
                })
            },
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
                   fs.parent, \
                   fs.basename, \
                   fs.fs_type, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.hash, \
                   fs.s_data, \
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

    pub fn is_file(&self) -> bool {
        match self {
            Self::File(_) => true,
            _ => false
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

    pub fn storage_id(&self) -> &ids::StorageId {
        match self {
            Self::Root(root) => root.storage.id(),
            Self::Directory(dir) => dir.storage.id(),
            Self::File(file) => file.storage.id(),
        }
    }

    pub fn into_schema(self) -> schema::fs::Item {
        match self {
            Self::Root(root) => schema::fs::Item::Root(root.into_schema()),
            Self::Directory(dir) => schema::fs::Item::Directory(dir.into_schema()),
            Self::File(file) => schema::fs::Item::File(file.into_schema()),
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
}

impl traits::Common for Item {
    fn id(&self) -> &ids::FSId {
        Self::id(self)
    }

    fn full_path(&self) -> PathBuf {
        match self {
            Self::Root(root) => root.full_path(),
            Self::Directory(dir) => dir.full_path(),
            Self::File(file) => file.full_path(),
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
