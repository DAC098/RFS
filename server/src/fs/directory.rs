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
use crate::util::sql;

use super::consts;
use super::traits;
use super::error::{StreamError, BuilderError};
use super::{name_check, name_gen};

pub struct Builder<'a, 'b> {
    id: ids::FSId,
    user_id: ids::UserId,
    storage: &'a storage::Medium,
    parent: &'b dyn traits::Container,
    basename: Option<String>,
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

    pub async fn build(self, conn: &impl GenericClient) -> Result<Directory, BuilderError> {
        let created = chrono::Utc::now();
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

        let storage = match &self.storage.type_ {
            storage::types::Type::Local(local) => {
                let mut full = local.path.join(&path);
                full.set_file_name(&basename);

                tokio::fs::create_dir(full).await?;

                storage::fs::Storage::Local(storage::fs::Local {
                    id: self.storage.id.clone(),
                })
            }
        };

        {
            let storage_json = PgJson(&storage);
            let path_display = path.to_str().unwrap();

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
                    comment, \
                    created\
                ) values \
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &self.id,
                    &self.user_id,
                    &parent,
                    &basename,
                    &consts::DIR_TYPE,
                    &path_display,
                    &storage_json,
                    &self.comment,
                    &created
                ]
            ).await?;
        }

        tags::create_tags(conn, "fs_tags", "fs_id", &self.id, &self.tags).await?;

        Ok(Directory {
            id: self.id,
            user_id: self.user_id,
            storage,
            parent,
            basename,
            path,
            tags: self.tags,
            comment: self.comment,
            created,
            updated: None,
            deleted: None
        })
    }
}

pub struct Directory {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: storage::fs::Storage,
    pub parent: ids::FSId,
    pub basename: String,
    pub path: PathBuf,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: chrono::DateTime<chrono::Utc>,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

impl Directory {
    pub fn builder<'a, 'b, P>(
        id: ids::FSId,
        user_id: ids::UserId,
        storage: &'a storage::Medium,
        parent: &'b P,
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
            tags: tags::TagMap::new(),
            comment: None
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::FSId
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsVec = vec![id];
        let tags_params: sql::ParamsVec = vec![id];

        let record_query = conn.query_opt(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.parent, \
                   fs.basename, \
                   fs.fs_path, \
                   fs.comment, \
                   fs.s_data, \
                   fs.created, \
                   fs.updated, \
                   fs.deleted \
            from fs \
            where fs.id = $1 and fs_type = 2",
            record_params.as_slice()
        );
        let tags_query = conn.query_raw(
            "\
            select fs_tags.tag, \
                   fs_tags.value \
            from fs_tags \
                join fs on \
                    fs_tags.fs_id = fs.id \
            where fs.id = $1 and \
                  fs.fs_type = 2",
            tags_params
        );

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags_stream)) => {
                Ok(Some(Directory {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(6)),
                    parent: row.get(2),
                    basename: row.get(3),
                    path: sql::pathbuf_from_sql(row.get(4)),
                    tags: tags::from_row_stream(tags_stream).await?,
                    comment: row.get(5),
                    created: row.get(7),
                    updated: row.get(8),
                    deleted: row.get(9)
                }))
            },
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn into_model(self) -> models::fs::Directory {
        models::fs::Directory {
            id: self.id,
            user_id: self.user_id,
            storage: self.storage.into_model(),
            parent: self.parent,
            basename: self.basename,
            path: self.path,
            tags: self.tags,
            comment: self.comment,
            created: self.created,
            updated: self.updated,
            deleted: self.deleted,
        }
    }
}

impl traits::Common for Directory {
    fn id(&self) -> &ids::FSId {
        &self.id
    }

    fn full_path(&self) -> PathBuf {
        self.path.join(&self.basename)
    }
}

impl traits::Container for Directory {}
