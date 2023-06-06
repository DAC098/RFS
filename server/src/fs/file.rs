use std::path::{PathBuf, Path};
use std::fmt::Write;

use futures::TryStream;
use tokio_postgres::Error as PgError;
use tokio_postgres::types::Json as PgJson;
use deadpool_postgres::GenericClient;
use serde::{Serialize, Deserialize};
use lib::ids;
use lib::models;

use crate::net;
use crate::storage;
use crate::tags;
use crate::util::{sql, PgParams};

use super::error::{StreamError, BuilderError};
use super::checksum;
use super::stream;
use super::{FILE_TYPE, name_check, name_gen, IdOption};
use super::directory::Directory;

pub struct Builder<'a, 'b> {
    id: ids::FSId,
    user_id: ids::UserId,
    storage: &'a storage::Medium,
    parent: Option<&'b Directory>,
    basename: Option<String>,
    mime: Option<mime::Mime>,
    checksums: checksum::ChecksumBuilder,
    tags: tags::TagMap,
    comment: Option<String>,
}

impl<'a, 'b> Builder<'a, 'b> {
    pub fn parent<'c>(mut self, parent: &'c Directory) -> Builder<'a, 'c> {
        Builder {
            id: self.id,
            user_id: self.user_id,
            storage: self.storage,
            parent: Some(parent),
            basename: self.basename,
            mime: self.mime,
            checksums: self.checksums,
            tags: self.tags,
            comment: self.comment,
        }
    }

    pub fn basename<B>(&mut self, basename: B) -> ()
    where
        B: Into<String>
    {
        self.basename = Some(basename.into());
    }

    pub fn mime(&mut self, mime: mime::Mime) -> () {
        self.mime = Some(mime);
    }

    pub fn add_checksum<C>(&mut self, checksum: C) -> () 
    where
        C: checksum::Digest + Send + 'static
    {
        self.checksums.add(checksum);
    }

    pub fn add_tag<T, V>(&mut self, tag: T, value: Option<V>) -> ()
    where
        T: Into<String>,
        V: Into<String>,
    {
        if let Some(v) = value {
            self.tags.insert(tag.into(), Some(v.into()));
        } else {
            self.tags.insert(tag.into(), None);
        }
    }

    pub fn comment<C>(&mut self, comment: C) -> ()
    where
        C: Into<String>
    {
        self.comment = Some(comment.into())
    }

    pub async fn build<S>(
        self,
        conn: &impl GenericClient,
        mut stream: S
    ) -> Result<File, BuilderError>
    where
        S: TryStream + Unpin,
        S::Ok: AsRef<[u8]>,
        StreamError: From<<S as TryStream>::Error>
    {
        let created = chrono::Utc::now();
        let mime = self.mime.unwrap_or(mime::APPLICATION_OCTET_STREAM);
        let id_opt: IdOption;
        let path: PathBuf;
        let parent: Option<ids::FSId>;

        if let Some(dir) = self.parent {
            id_opt = IdOption::Parent(&dir.id);
            path = dir.path.join(&dir.basename);
            parent = Some(dir.id.clone());
        } else {
            id_opt = IdOption::Storage(&self.storage.id);
            path = PathBuf::new();
            parent = None;
        }

        let basename = if let Some(given) = self.basename {
            if !name_check(conn, &id_opt, &given).await?.is_some() {
                return Err(BuilderError::BasenameExists);
            }

            given
        } else {
            let Some(gen) = name_gen(conn, &id_opt, 100).await? else {
                return Err(BuilderError::BasenameGenFailed);
            };

            gen
        };

        let (size, storage) = stream::new_stream_file(
            path.join(&basename),
            &self.storage,
            stream
        ).await?;

        {
            let storage_json = PgJson(&storage);
            let path_display = path.to_str().unwrap();
            let mime_type = mime.type_().as_str();
            let mime_subtype = mime.subtype().as_str();

            let _ = conn.execute(
                "\
                insert into fs (\
                    id, \
                    user_id, \
                    parent, \
                    basename, \
                    fs_type, \
                    fs_path, \
                    s_data, \
                    mime_type, \
                    mime_subtype, \
                    comment, \
                    created\
                ) values \
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                &[
                    &self.id,
                    &self.user_id,
                    &parent,
                    &basename,
                    &FILE_TYPE,
                    &path_display,
                    &storage_json,
                    &mime_type,
                    &mime_subtype,
                    &self.comment,
                    &created
                ]
            ).await?;
        }

        tags::create_tags(conn, "fs_tags", "fs_id", &self.id, &self.tags).await?;

        Ok(File {
            id: self.id,
            user_id: self.user_id,
            storage,
            parent,
            basename,
            path,
            mime,
            size,
            tags: self.tags,
            comment: self.comment,
            created,
            updated: None,
            deleted: None,
        })
    }
}

pub struct File {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: storage::fs::Storage,
    pub parent: Option<ids::FSId>,
    pub path: PathBuf,
    pub basename: String,
    pub mime: mime::Mime,
    pub size: u64,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: chrono::DateTime<chrono::Utc>,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

impl File {
    pub fn builder<'a>(
        id: ids::FSId, 
        user_id: ids::UserId, 
        storage: &'a storage::Medium
    ) -> Builder<'a, 'static> {
        Builder {
            id,
            user_id,
            storage,
            parent: None,
            basename: None,
            mime: None,
            checksums: checksum::ChecksumBuilder::new(),
            tags: tags::TagMap::new(),
            comment: None
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::FSId
    ) -> Result<Option<Self>, PgError> {
        let mut record_params = PgParams::with_capacity(1);
        record_params.push(id);

        let record_query = conn.query_opt(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.parent, \
                   fs.basename, \
                   fs.fs_path, \
                   fs.fs_size, \
                   fs.mime_type, \
                   fs.mime_subtype, \
                   fs.comment, \
                   fs.s_data, \
                   fs.created, \
                   fs.updated, \
                   fs.deleted \
            from fs \
            where fs.id = $1 and fs_type = 1",
            record_params.as_slice()
        );
        let tags_query = conn.query(
            "\
            select fs_tags.tag, \
                   fs_tags.value \
            from fs_tags \
                join fs on \
                    fs_tags.fs_id = fs.id \
            where fs.id = $1 and \
                  fs.fs_type = 1",
            record_params.as_slice()
        );

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags_list)) => {
                let mut tags = tags::TagMap::with_capacity(tags_list.len());

                for tag_row in tags_list {
                    tags.insert(tag_row.get(0), tag_row.get(1));
                }

                Ok(Some(File {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(9)),
                    parent: row.get(3),
                    path: sql::pathbuf_from_sql(row.get(5)),
                    basename: row.get(4),
                    mime: sql::mime_from_sql(row.get(7), row.get(8)),
                    size: sql::u64_from_sql(row.get(6)),
                    tags: tags,
                    comment: row.get(9),
                    created: row.get(10),
                    updated: row.get(11),
                    deleted: row.get(12),
                }))
            },
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn into_model(self) -> models::fs::File {
        models::fs::File {
            id: self.id,
            user_id: self.user_id,
            parent: self.parent,
            basename: self.basename,
            path: self.path,
            size: self.size,
            mime: self.mime,
            tags: self.tags,
            comment: self.comment,
            checksums: Vec::new(),
            storage: self.storage.into_model(),
            created: self.created,
            updated: self.updated,
            deleted: self.deleted,
        }
    }

    pub fn into_model_item(self) -> models::fs::Item {
        models::fs::Item::File(self.into_model())
    }
}
