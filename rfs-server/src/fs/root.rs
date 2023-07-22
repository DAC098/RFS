use std::path::PathBuf;

use rfs_lib::ids;
use rfs_lib::schema;
use chrono::{DateTime, Utc};
use tokio_postgres::Error as PgError;
use deadpool_postgres::GenericClient;

use crate::storage;
use crate::util::sql;
use crate::tags;

use super::traits;

#[derive(Debug)]
pub struct Root {
    pub id: ids::FSId,
    pub user_id: ids::UserId,
    pub storage: storage::fs::Storage,
    pub tags: tags::TagMap,
    pub comment: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl Root {
    pub async fn storage_id_retrieve(
        conn: &impl GenericClient,
        id: &ids::StorageId
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsVec = vec![id];
        let tags_params: sql::ParamsVec = vec![id];

        let record_query = conn.query_opt(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.comment, \
                   fs.s_data, \
                   fs.created, \
                   fs.updated, \
                   fs.deleted \
            from fs \
            where fs.s_data->>'id' = $1 and \
                  fs.fs_type = 0",
            record_params.as_slice()
        );

        let options = tags::GetTagsOptions::new()
            .with_join("join fs on fs_tags.fs_id = fs.id")
            .with_where("fs.s_data->>'id' = $1 and fs.fs_type = 1")
            .with_params(tags_params);
        let tags_query = tags::get_tags_options(conn, "fs_tags", options);

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags)) => {
                Ok(Some(Root {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(3)),
                    tags,
                    comment: row.get(2),
                    created: row.get(3),
                    updated: row.get(4),
                    deleted: row.get(5),
                }))
            },
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::FSId
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsVec = vec![id];

        let record_query = conn.query_opt(
            "\
            select fs.id, \
                   fs.user_id, \
                   fs.comment, \
                   fs.s_data, \
                   fs.created, \
                   fs.updated, \
                   fs.deleted \
            from fs \
            where fs.id = $1 and fs_type = 0",
            record_params.as_slice()
        );
        let options = tags::GetTagsOptions::new()
            .with_join("join fs on fs_tags.fs_id = fs.id")
            .with_where("and fs.fs_type = 1")
            .with_id_field("fs_id", id);
        let tags_query = tags::get_tags_options(conn, "fs_tags", options);

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags)) => {
                Ok(Some(Root {
                    id: row.get(0),
                    user_id: row.get(1),
                    storage: sql::de_from_sql(row.get(3)),
                    tags,
                    comment: row.get(2),
                    created: row.get(4),
                    updated: row.get(5),
                    deleted: row.get(6)
                }))
            }
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn into_schema(self) -> schema::fs::Root {
        schema::fs::Root {
            id: self.id,
            user_id: self.user_id,
            storage: self.storage.into_schema(),
            tags: self.tags,
            comment: self.comment,
            created: self.created,
            updated: self.updated,
            deleted: self.deleted,
        }
    }
}

impl traits::Common for Root {
    fn id(&self) -> &ids::FSId {
        &self.id
    }

    fn full_path(&self) -> PathBuf {
        PathBuf::new()
    }
}

impl traits::Container for Root {}
