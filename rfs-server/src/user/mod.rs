use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;

pub struct UserEmail {
    email: String,
    verified: bool
}

pub struct User {
    id: ids::UserId,
    username: String,
    email: Option<UserEmail>,
}

impl User {
    pub fn id(&self) -> &ids::UserId {
        &self.id
    }

    pub fn username(&self) -> &String {
        &self.username
    }

    pub fn email(&self) -> Option<&UserEmail> {
        self.email.as_ref()
    }
}

impl User {
    pub async fn query_with_id(conn: &impl GenericClient, id: &ids::UserId) -> Result<Option<User>, PgError> {
        if let Some(row) = conn.query_opt(
            "\
            select users.id, \
                   users.username, \
                   users.email, \
                   users.email_verified \
            from users \
            where users.id = $1",
            &[id]
        ).await? {
            let email = if let Some(email) = row.get(2) {
                Some(UserEmail {
                    email,
                    verified: row.get(3),
                })
            } else {
                None
            };

            Ok(Some(User {
                id: ids::user_id_from_pg(row.get(0)),
                username: row.get(1),
                email,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn query_with_username(conn: &impl GenericClient, username: &String) -> Result<Option<User>, PgError> {
        if let Some(row) = conn.query_opt(
            "\
            select users.id, \
                   users.username, \
                   users.email, \
                   users.email_verified \
            from users \
            where users.username = $1",
            &[username]
        ).await? {
            let email = if let Some(email) = row.get(2) {
                Some(UserEmail {
                    email,
                    verified: row.get(3)
                })
            } else {
                None
            };

            Ok(Some(User {
                id: ids::user_id_from_pg(row.get(0)),
                username: row.get(1),
                email
            }))
        } else {
            Ok(None)
        }
    }
}
