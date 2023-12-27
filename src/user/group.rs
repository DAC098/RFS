use rfs_lib::ids;
use chrono::{DateTime, Utc};
use deadpool_postgres::GenericClient;
use tokio_postgres::{Error as PgError};

pub struct Group {
    pub id: ids::GroupId,
    pub name: String,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
}

impl Group {
    pub async fn retrieve(
        conn: &impl GenericClient,
        group_id: &ids::GroupId,
    ) -> Result<Option<Self>, PgError> {
        Ok(conn.query_opt(
            "select id, name, created, updated from groups where id = $1",
            &[group_id]
        )
            .await?
            .map(|row| Group {
                id: row.get(0),
                name: row.get(1),
                created: row.get(2),
                updated: row.get(3),
            }))
    }
}
