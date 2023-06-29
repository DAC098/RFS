use std::path::PathBuf;

use lib::ids;
use lib::schema::storage::{StorageItem, StorageType};
use chrono::{DateTime, Utc};
use tokio_postgres::Error as PgError;
use tokio_postgres::types::Json as PgJson;
use deadpool_postgres::GenericClient;
use futures::TryStreamExt;
use serde::{Serialize, Deserialize};

use crate::util::sql;
use crate::tags;

pub mod error;
pub mod fs;
pub mod types;

use error::BuilderError;

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

pub struct MediumBuilder {
    id: ids::StorageId,
    name: String,
    user_id: ids::UserId,
    tags: tags::TagMap,
    type_: types::Type,
}

impl MediumBuilder {
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

    pub fn set_tags(&mut self, tags: tags::TagMap) -> () {
        self.tags = tags;
    }

    pub async fn build(self, conn: &impl GenericClient) -> Result<Medium, BuilderError> {
        let created = Utc::now();

        if name_check(conn, &self.user_id, &self.name).await?.is_some() {
            return Err(BuilderError::NameExists);
        }

        {
            let storage_json = PgJson(&self.type_);

            conn.execute(
                "\
                insert into storage (id, user_id, name, s_data, created) values \
                ($1, $2, $3, $4, $5)",
                &[&self.id, &self.user_id, &self.name, &storage_json, &created]
            ).await?;

            tags::create_tags(conn, "storage_tags", "storage_id", &self.id, &self.tags).await?;
        }

        Ok(Medium {
            id: self.id,
            name: self.name,
            user_id: self.user_id,
            type_: self.type_,
            tags: self.tags,
            created,
            updated: None,
            deleted: None
        })
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
    pub fn builder<T>(
        id: ids::StorageId,
        user_id: ids::UserId,
        name: String,
        type_: T,
    ) -> MediumBuilder
    where
        T: Into<types::Type>
    {
        MediumBuilder {
            id,
            name,
            user_id,
            type_: type_.into(),
            tags: tags::TagMap::new(),
        }
    }

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
                    created: row.get(5),
                    updated: row.get(6),
                    deleted: row.get(7),
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

