use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use argon2::Variant;
use rand::RngCore;

use crate::net;
use crate::sec::secret::Secret;

pub const SALT_LEN: usize = 32;

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
    password: String,
    secret: &'a Secret
}

impl<'a> PasswordBuilder<'a> {
    pub async fn build(self, conn: &impl GenericClient) -> Result<Password, PasswordError> {
        let version = *self.secret.version();

        let mut config = argon2::Config::default();
        config.mem_cost = 19456;
        config.variant = Variant::Argon2id;
        config.secret = self.secret.as_slice();

        let mut salt = [0u8; SALT_LEN];

        rand::thread_rng().try_fill_bytes(salt.as_mut_slice())?;

        let hash = argon2::hash_encoded(
            self.password.as_bytes(),
            &salt.as_slice(),
            &config
        )?;

        let _ = conn.execute(
            "\
            insert into auth_password (user_id, version, hash) values
            ($1, $2, $3)",
            &[&self.user_id, &(version as i32), &hash]
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
    pub fn builder<'a>(
        user_id: ids::UserId,
        password: String,
        secret: &'a Secret,
    ) -> PasswordBuilder<'a> {
        PasswordBuilder {
            user_id,
            password,
            secret,
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

    pub async fn update<P>(
        &mut self,
        conn: &impl GenericClient,
        password: P,
        secret: &Secret
    ) -> Result<(), PasswordError>
    where
        P: AsRef<[u8]>
    {
        self.version = *secret.version();

        let mut config = argon2::Config::default();
        config.mem_cost = 19456;
        config.variant = Variant::Argon2id;
        config.secret = secret.as_slice();

        let mut salt = [0u8; SALT_LEN];

        rand::thread_rng().try_fill_bytes(salt.as_mut_slice())?;

        let hash = argon2::hash_encoded(
            password.as_ref(),
            &salt,
            &config
        )?;

        let _ = conn.execute(
            "update auth_password set hash = $2, version = $3 where user_id = $1",
            &[&self.user_id, &hash, &(self.version as i32)]
        );

        Ok(())
    }

    pub async fn delete(&self, conn: &impl GenericClient) -> Result<bool, PgError> {
        let deleted = conn.execute(
            "delete from auth_password where user_id = $1",
            &[&self.user_id]
        ).await?;

        Ok(deleted == 1)
    }
}
