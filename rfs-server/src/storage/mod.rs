use rfs_lib::ids;
use rfs_lib::schema::storage::StorageItem;
use chrono::{DateTime, Utc};
use tokio_postgres::Error as PgError;
use deadpool_postgres::GenericClient;

use crate::sql;
use crate::tags;

pub mod fs;
pub mod types;

pub async fn name_check<N>(
    conn: &impl GenericClient,
    user_id: &ids::UserId,
    name: N
) -> Result<Option<ids::StorageId>, PgError>
where
    N: AsRef<str>
{
    if let Some(row) = conn.query_opt(
        "select id from storage where name = $1 and user_id = $2",
        &[&name.as_ref(), user_id]
    ).await? {
        Ok(row.get(0))
    } else {
        Ok(None)
    }
}

pub struct Medium {
    pub id: ids::StorageId,
    pub name: String,
    pub user_id: ids::UserId,
    pub type_: types::Type,
    pub tags: tags::TagMap,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl Medium {
    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::StorageId,
    ) -> Result<Option<Self>, PgError> {
        let record_params: sql::ParamsVec = vec![id];
        let tags_params: sql::ParamsVec = vec![id];

        let record_query = conn.query_opt(
            "\
            select storage.id, \
                   storage.user_id, \
                   storage.name, \
                   storage.s_data, \
                   storage.created, \
                   storage.updated, \
                   storage.deleted \
            from storage \
            where storage.id = $1",
            record_params.as_slice()
        );
        let tags_query = conn.query_raw(
            "\
            select storage_tags.tag, \
                   storage_tags.value \
            from storage_tags \
                join storage on \
                    storage_tags.storage_id = storage.id \
            where storage.id = $1",
            tags_params
        );

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), tags_stream)) => {
                Ok(Some(Medium {
                    id: row.get(0),
                    user_id: row.get(1),
                    name: row.get(2),
                    type_: sql::de_from_sql(row.get(3)),
                    tags: tags::from_row_stream(tags_stream).await?,
                    created: row.get(4),
                    updated: row.get(5),
                    deleted: row.get(6),
                }))
            },
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn into_schema(self) -> StorageItem {
        StorageItem {
            id: self.id,
            name: self.name,
            user_id: self.user_id,
            type_: self.type_.into_schema(),
            tags: self.tags,
            created: self.created,
            updated: self.updated,
            deleted: self.deleted
        }
    }
}

