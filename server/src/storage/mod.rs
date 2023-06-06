use std::path::PathBuf;

use lib::ids;
use lib::models::storage::{StorageItem, StorageType};
use chrono::{DateTime, Utc};
use tokio_postgres::Error as PgError;
use tokio_postgres::types::Json as PgJson;
use deadpool_postgres::GenericClient;
use futures::TryStreamExt;
use serde::{Serialize, Deserialize};

use crate::util::{sql, ParamsVec, PgParams};
use crate::tags;

pub mod fs;
pub mod types;

#[derive(Debug, Serialize, Deserialize)]
pub enum Type {
    Local(types::Local)
}

impl Type {
    pub fn into_model(self) -> StorageType {
        match self {
            Type::Local(local) => {
                StorageType::Local {
                    path: local.path
                }
            }
        }
    }
}

impl From<types::Local> for Type {
    fn from(local: types::Local) -> Self {
        Type::Local(local)
    }
}

pub struct MediumBuilder {
    id: ids::StorageId,
    name: String,
    user_id: ids::UserId,
    tags: tags::TagMap,
}

pub struct Medium {
    pub id: ids::StorageId,
    pub name: String,
    pub user_id: ids::UserId,
    pub type_: Type,
    pub tags: tags::TagMap,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub deleted: Option<DateTime<Utc>>,
}

impl Medium {
    pub async fn retrieve(
        conn: &impl GenericClient,
        user_id: &ids::UserId,
        id: &ids::StorageId,
    ) -> Result<Option<Self>, PgError> {
        let record_params: ParamsVec = vec![id, user_id];
        let tags_params = vec![id, user_id];

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
            where storage.id = $1 and \
                  storage.user_id = $2",
            record_params.as_slice()
        );
        let tags_query = conn.query_raw(
            "\
            select storage_tags.tag, \
                   storage_tags.value \
            from storage_tags \
                join storage on \
                    storage_tags.storage_id = storage.id \
            where storage.id = $1 and \
                  storage.user_id = $2",
            tags_params
        );

        match tokio::try_join!(record_query, tags_query) {
            Ok((Some(row), mut tags_stream)) => {
                futures::pin_mut!(tags_stream);

                let mut tags = tags::TagMap::new();

                while let Some(row) = tags_stream.try_next().await? {
                    tags.insert(row.get(0), row.get(1));
                }

                Ok(Some(Medium {
                    id: row.get(0),
                    user_id: row.get(1),
                    name: row.get(2),
                    type_: sql::de_from_sql(row.get(3)),
                    tags,
                    created: row.get(5),
                    updated: row.get(6),
                    deleted: row.get(7),
                }))
            },
            Ok((None, _)) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub fn into_model(self) -> StorageItem {
        StorageItem {
            id: self.id,
            name: self.name,
            user_id: self.user_id,
            type_: self.type_.into_model(),
            tags: self.tags,
            created: self.created,
            updated: self.updated,
            deleted: self.deleted
        }
    }
}

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

pub async fn exists_check(
    conn: &impl GenericClient,
    user_id: &ids::UserId,
    id: &ids::StorageId,
    deleted: Option<bool>,
) -> Result<bool, PgError> {
    let check = if let Some(b) = deleted {
        let query = if b {
            "\
            select id \
            from storage \
            where id = $1 and \
                  user_id = $2 and \
                  deleted is null"
        } else {
            "\
            select id \
            from storage \
            where id = $1 and \
                  user_id = $2 and \
                  deleted is not null"
        };

        conn.execute(query, &[id, user_id]).await?
    } else {
        conn.execute(
            "select id from storage where id = $1 and user_id = $2",
            &[id, user_id]
        ).await?
    };

    Ok(check == 1)
}
