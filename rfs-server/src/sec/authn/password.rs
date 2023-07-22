use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;

use crate::net;
use crate::sec::secret::{EMPTY_SECRET, Secret};

pub enum PasswordError {
    Rand(rand::Error),
    Argon2(argon2::Error),
    Pg(PgError),
}

impl From<rand::Error> for PasswordError {
    fn from(err: rand::Error) -> Self {
        PasswordError::Rand(err)
    }
}

impl From<argon2::Error> for PasswordError {
    fn from(err: argon2::Error) -> Self {
        PasswordError::Argon2(err)
    }
}

impl From<PgError> for PasswordError {
    fn from(err: PgError) -> Self {
        PasswordError::Pg(err)
    }
}

impl From<PasswordError> for net::error::Error {
    fn from(err: PasswordError) -> net::error::Error {
        match err {
            PasswordError::Rand(err) => err.into(),
            PasswordError::Argon2(err) => err.into(),
            PasswordError::Pg(err) => err.into(),
        }
    }
}

pub struct PasswordBuilder<'a> {
    user_id: ids::UserId,
    salt_len: usize,
    secret: Option<&'a Secret>
}

impl<'a> PasswordBuilder<'a> {
    pub fn with_salt_len(mut self, len: usize) -> Self {
        self.salt_len = len;
        self
    }

    pub fn with_secret<'b>(self, secret: &'b Secret) -> PasswordBuilder<'b> {
        PasswordBuilder {
            user_id: self.user_id,
            salt_len: self.salt_len,
            secret: Some(secret)
        }
    }

    pub async fn build(self, conn: &impl GenericClient) -> Result<Password, PasswordError> {
        use argon2::{Variant};
        use rand::RngCore;

        let secret = self.secret.unwrap_or(&EMPTY_SECRET);
        let version = *secret.version();

        let mut config = argon2::Config::default();
        config.mem_cost = 19456;
        config.secret = secret.as_slice();
        config.variant = Variant::Argon2id;

        let mut salt = Vec::with_capacity(self.salt_len);

        for _ in 0..salt.len() {
            salt.push(0);
        }

        rand::thread_rng()
            .try_fill_bytes(salt.as_mut_slice())?;

        let hash = argon2::hash_encoded(
            secret.as_slice(),
            &salt.as_slice(),
            &config
        )?;

        let _ = conn.execute(
            "\
            insert into auth_password (user_id, version, hash) values
            ($1, $2, $3)",
            &[
                &self.user_id,
                &(version as i32),
                &hash
            ]
        ).await?;

        Ok(Password {
            user_id: self.user_id,
            version,
            hash,
        })
    }
}

pub struct Password {
    user_id: ids::UserId,
    version: u32,
    hash: String
}

impl Password {
    pub fn builder(user_id: ids::UserId) -> PasswordBuilder<'static> {
        PasswordBuilder {
            user_id,
            salt_len: 32,
            secret: None,
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        user_id: &ids::UserId,
    ) -> Result<Option<Password>, PgError> {
        if let Some(row) = conn.query_opt(
            "\
            select auth_password.user_id, \
                   auth_password.version, \
                   auth_password.hash \
            from auth_password \
            where auth_password.user_id = $1",
            &[user_id]
        ).await? {
            Ok(Some(Password {
                user_id: row.get(0),
                version: row.get(1),
                hash: row.get(2)
            }))
        } else {
            Ok(None)
        }
    }

    pub fn version(&self) -> &u32 {
        &self.version
    }

    pub fn verify<C>(&self, check: C, secret: &Secret) -> Result<bool, PasswordError>
    where
        C: AsRef<[u8]>,
    {
        let ad = [0u8; 0];

        let result = argon2::verify_encoded_ext(
            &self.hash.as_str(),
            check.as_ref(),
            secret.as_slice(),
            &ad
        )?;

        Ok(result)
    }

    pub async fn delete(&self, conn: &impl GenericClient) -> Result<bool, PgError> {
        let deleted = conn.execute(
            "delete from auth_password where user_id = $1",
            &[&self.user_id]
        ).await?;

        Ok(deleted == 1)
    }
}
