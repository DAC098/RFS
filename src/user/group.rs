use rfs_lib::ids;
use chrono::{DateTime, Utc};
use deadpool_postgres::GenericClient;
use tokio_postgres::{Error as PgError};

pub struct Group {
    pub id: ids::GroupSet,
    pub name: String,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
}

impl Group {
    pub async fn retrieve_uid(
        conn: &impl GenericClient,
        group_uid: &ids::GroupUid,
    ) -> Result<Option<Self>, PgError> {
        Ok(conn.query_opt(
            "select id, name, create, updated from groups where uid = $1",
            &[group_uid]
        ).await?.map(|row| Group {
            id: ids::GroupSet::new(row.get(0), group_uid.clone()),
            name: row.get(1),
            created: row.get(2),
            updated: row.get(3),
        }))
    }
}
