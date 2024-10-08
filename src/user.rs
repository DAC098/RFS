use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;

pub mod group;

pub async fn check_username<U>(
    conn: &impl GenericClient,
    username: U,
) -> Result<Option<ids::UserId>, PgError>
where
    U: AsRef<str>
{
    let username_ref = username.as_ref();

    if let Some(row) = conn.query_opt(
        "select id from users where username = $1",
        &[&username_ref]
    ).await? {
        Ok(Some(row.get(0)))
    } else {
        Ok(None)
    }
}

pub async fn check_email<E>(
    conn: &impl GenericClient,
    email: E,
) -> Result<Option<ids::UserId>, PgError>
where
    E: AsRef<str>
{
    let email_ref = email.as_ref();

    if let Some(row) = conn.query_opt(
        "select id from users where email = $1",
        &[&email_ref]
    ).await? {
        Ok(Some(row.get(0)))
    } else {
        Ok(None)
    }
}

pub async fn check_username_and_email<U, E>(
    conn: &impl GenericClient,
    username: U,
    email: E
) -> Result<(Option<ids::UserId>, Option<ids::UserId>), PgError>
where
    U: AsRef<str>,
    E: AsRef<str>,
{
    let username_ref = username.as_ref();
    let email_ref = email.as_ref();

    let results = conn.query(
        "\
        select id, \
               username = $1 as is_username, \
               email = $2 as is_email \
        from users \
        where username = $1 or \
              email = $2",
        &[&username_ref, &email_ref]
    ).await?;

    let mut username_id = None;
    let mut email_id = None;

    for row in results {
        let id: ids::UserId = row.get(0);
        let is_username: bool = row.get(1);
        let is_email: bool = row.get(2);

        if is_username && is_email {
            username_id = Some(id.clone());
            email_id = Some(id);
        } else if is_username {
            username_id = Some(id);
        } else {
            email_id = Some(id);
        }
    }

    Ok((username_id, email_id))
}

#[derive(Debug, Clone)]
pub struct UserEmail {
    pub email: String,
    pub verified: bool
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: ids::UserSet,
    pub username: String,
    pub email: Option<UserEmail>,
}

impl User {
    pub fn id(&self) -> &ids::UserId {
        self.id.local()
    }
}

impl User {
    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::UserId
    ) -> Result<Option<User>, PgError> {
        Ok(conn.query_opt(
            "\
            select users.uid, \
                   users.username, \
                   users.email, \
                   users.email_verified \
            from users \
            where users.id = $1",
            &[id]
        ).await?.map(|row| User {
            id: ids::UserSet::new(id.clone(), row.get(0)),
            username: row.get(1),
            email: if let Some(email) = row.get(2) {
                Some(UserEmail {
                    email,
                    verified: row.get(3),
                })
            } else {
                None
            }
        }))
    }

    pub async fn retrieve_uid(
        conn: &impl GenericClient,
        uid: &ids::UserUid,
    ) -> Result<Option<Self>, PgError> {
        Ok(conn.query_opt(
            "\
            select users.id, \
                   users.username, \
                   users.email, \
                   users.email_verified \
            from users \
            where users.uid = $1",
            &[uid]
        ).await?.map(|row| User {
            id: ids::UserSet::new(row.get(0), uid.clone()),
            username: row.get(1),
            email: if let Some(email) = row.get(2) {
                Some(UserEmail {
                    email,
                    verified: row.get(3),
                })
            } else {
                None
            }
        }))
    }

    pub async fn query_with_id(conn: &impl GenericClient, id: &ids::UserId) -> Result<Option<User>, PgError> {
        User::retrieve(conn, id).await
    }

    pub async fn retrieve_username<U>(
        conn: &impl GenericClient,
        username: U
    ) -> Result<Option<User>, PgError>
    where
        U: AsRef<str>
    {
        let username_ref = username.as_ref();

        Ok(conn.query_opt(
            "\
            select users.id, \
                   users.uid, \
                   users.username, \
                   users.email, \
                   users.email_verified \
            from users \
            where users.username = $1",
            &[&username_ref]
        ).await?.map(|row| User {
            id: ids::UserSet::new(row.get(0), row.get(1)),
            username: row.get(2),
            email: if let Some(email) = row.get(3) {
                Some(UserEmail {
                    email,
                    verified: row.get(4),
                })
            } else {
                None
            }
        }))
    }

    pub async fn query_with_username(conn: &impl GenericClient, username: &String) -> Result<Option<User>, PgError> {
        User::retrieve_username(conn, username).await
    }
}
