use std::path::PathBuf;

use rfs_lib::ids;
use tokio_postgres::Error as PgError;
use deadpool_postgres::GenericClient;
use chrono::{DateTime, Utc};

use crate::storage;
use crate::tags;
use crate::sql;

use super::traits;

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

    pub fn into_schema(self) -> rfs_api::fs::File {
        rfs_api::fs::File {
            id: self.id,
            user_id: self.user_id,
            parent: self.parent,
            basename: self.basename,
            path: self.path,
            size: self.size,
            mime: self.mime,
            tags: self.tags,
            comment: self.comment,
            hash: self.hash.as_bytes().to_vec(),
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
