use std::path::PathBuf;

use rfs_lib::ids;
use rfs_lib::schema;
use tokio_postgres::Error as PgError;
use deadpool_postgres::GenericClient;
use chrono::{DateTime, Utc};

use crate::storage;
use crate::tags;
use crate::util::sql;

//use super::consts;
use super::traits;
//use super::error::{StreamError, BuilderError};
//use super::checksum;
//use super::stream;
//use super::{name_check, name_gen};

/*
pub struct Builder<'a, 'b> {
    id: ids::FSId,
    user_id: ids::UserId,
    storage: &'a storage::Medium,
    parent: &'b dyn traits::Container,
    basename: Option<String>,
    mime: Option<mime::Mime>,
    checksums: checksum::ChecksumBuilder,
    tags: tags::TagMap,
    comment: Option<String>,
}

impl<'a, 'b> Builder<'a, 'b> {
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
        let created = Utc::now();
        let mime = self.mime.unwrap_or(mime::APPLICATION_OCTET_STREAM);
        let path = self.parent.full_path();
        let parent = self.parent.id().clone();

        let basename = if let Some(given) = self.basename {
            if !name_check(conn, &parent, &given).await?.is_some() {
                return Err(BuilderError::BasenameExists);
            }

            given
        } else {
            let Some(gen) = name_gen(conn, &parent, 100).await? else {
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
                    &consts::FILE_TYPE,
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
*/

#[derive(Debug)]
pub struct File {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: storage::fs::Storage,
    pub parent: ids::FSId,
    pub path: PathBuf,
    pub basename: String,
    pub mime: mime::Mime,
    pub size: u64,
    pub hash: blake3::Hash,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl File {
    /*
    pub fn builder<'a, 'b, P>(
        id: ids::FSId,
        user_id: ids::UserId,
        storage: &'a storage::Medium,
        parent: &'b P
    ) -> Builder<'a, 'b>
    where
        P: traits::Container
    {
        Builder {
            id,
            user_id,
            storage,
            parent,
            basename: None,
            mime: None,
            checksums: checksum::ChecksumBuilder::new(),
            tags: tags::TagMap::new(),
            comment: None
        }
    }
    */

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::FSId
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsVec = vec![id];

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
                   fs.hash, \
                   fs.comment, \
                   fs.s_data, \
                   fs.created, \
                   fs.updated, \
                   fs.deleted \
            from fs \
            where fs.id = $1 and fs_type = 1",
            record_params.as_slice()
        );
        let options = tags::GetTagsOptions::new()
            .with_join("join fs on fs_tags.fs_id = fs.id")
            .with_where("and fs.fs_type = 1")
            .with_id_field("fs_id", id);
        let tags_query = tags::get_tags_options(conn, "fs_tags", options);

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags)) => {
                Ok(Some(File {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(9)),
                    parent: row.get(3),
                    path: sql::pathbuf_from_sql(row.get(5)),
                    basename: row.get(4),
                    mime: sql::mime_from_sql(row.get(7), row.get(8)),
                    size: sql::u64_from_sql(row.get(6)),
                    hash: sql::blake3_hash_from_sql(row.get(9)),
                    tags,
                    comment: row.get(10),
                    created: row.get(11),
                    updated: row.get(12),
                    deleted: row.get(13),
                }))
            },
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn into_schema(self) -> schema::fs::File {
        schema::fs::File {
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
            storage: self.storage.into_schema(),
            created: self.created,
            updated: self.updated,
            deleted: self.deleted,
        }
    }
}

impl traits::Common for File {
    fn id(&self) -> &ids::FSId {
        &self.id
    }

    fn full_path(&self) -> PathBuf {
        self.path.join(&self.basename)
    }
}
