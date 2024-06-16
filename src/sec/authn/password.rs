use rfs_lib::ids;
use rfs_lib::sec::chacha;

use base64::{Engine, engine::general_purpose::STANDARD};
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use argon2::Variant;
use rand::RngCore;

use crate::net::error::Error as NetError;
use crate::sec::secrets::{PeppersManager, PMError};
use crate::sql;

pub const SALT_LEN: usize = 32;

pub type Salt = [u8; SALT_LEN];

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("missing pepper for encrypt")]
    MissingPepper,

    #[error("failed updating password")]
    UpdateFailed,

    #[error("failed creating password")]
    CreateFailed,

    #[error("failed deleting password")]
    DeleteFailed,

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    Manager(#[from] PMError),

    #[error(transparent)]
    Rand(#[from] rand::Error),

    #[error(transparent)]
    Crypto(#[from] chacha::CryptoError),

    #[error(transparent)]
    Argon2(#[from] argon2::Error),

    #[error(transparent)]
    Db(#[from] PgError)
}

impl From<PasswordError> for NetError {
    fn from(err: PasswordError) -> Self {
        NetError::new().source(err)
    }
}

pub fn gen_salt() -> Result<Salt, rand::Error> {
    let mut salt = [0u8; SALT_LEN];

    rand::thread_rng().try_fill_bytes(&mut salt)?;

    Ok(salt)
}

pub fn gen_hash(password: &str, salt: &[u8]) -> Result<String, argon2::Error> {
    let mut config = argon2::Config::default();
    config.mem_cost = 19456;
    config.variant = Variant::Argon2id;

    Ok(argon2::hash_encoded(
        password.as_bytes(),
        salt,
        &config
    )?)
}

pub fn gen_encrypted(hash: String, manager: &PeppersManager) -> Result<(u64, String), PasswordError> {
    let bytes = hash.into_bytes();

    let (v,e) = manager.latest_cb(|result| {
        if let Some((ver, key)) = result? {
            Ok::<_, PasswordError>((*ver, chacha::encrypt_data(key.data(), bytes)?))
        } else {
            Ok::<_, PasswordError>((0, bytes))
        }
    })?;

    Ok((v, STANDARD.encode(e)))
}

pub struct Password {
    pub user_id: ids::UserId,
    pub version: u64,
    pub hash: String,
}

impl Password {
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
                version: sql::u64_from_sql(row.get(1)),
                hash: row.get(2)
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create(
        conn: &impl GenericClient,
        user_id: &ids::UserId,
        password: String,
        manager: &PeppersManager,
    ) -> Result<Self, PasswordError> {
        let salt = gen_salt()?;
        let (version, encrypted) = gen_encrypted(
            gen_hash(&password, &salt)?,
            manager
        )?;

        let result = conn.execute(
            "\
            insert into auth_password (user_id, version, hash) values \
            ($1, $2, $3)",
            &[user_id, &(version as i64), &encrypted]
        ).await?;

        if result != 1 {
            return Err(PasswordError::CreateFailed);
        }

        Ok(Password {
            user_id: user_id.clone(),
            hash: encrypted,
            version,
        })
    }

    pub async fn update(
        &mut self,
        conn: &impl GenericClient,
        update: String,
        manager: &PeppersManager,
    ) -> Result<(), PasswordError> {
        let salt = gen_salt()?;
        let (version, encrypted) = gen_encrypted(
            gen_hash(&update, &salt)?,
            manager
        )?;

        let result = conn.execute(
            "update auth_password set hash = $2, version = $3 where user_id = $1",
            &[&self.user_id, &encrypted, &(version as i64)]
        ).await?;

        if result != 1 {
            return Err(PasswordError::UpdateFailed);
        }

        self.hash = encrypted;
        self.version = version;

        Ok(())
    }

    pub fn verify<C>(&self, check: C, manager: &PeppersManager) -> Result<bool, PasswordError>
    where
        C: AsRef<[u8]>
    {
        let decoded = STANDARD.decode(&self.hash).unwrap();

        let result = if self.version != 0 {
            let decrypted = manager.get_cb(&self.version, |result| {
                if let Some(key) = result? {
                    Ok(chacha::decrypt_data(key.data(), decoded)?)
                } else {
                    Err(PasswordError::MissingPepper)
                }
            })?;

            let hash = std::str::from_utf8(&decrypted)?;

            argon2::verify_encoded_ext(hash, check.as_ref(), &[], &[])?
        } else {
            let hash = std::str::from_utf8(&decoded)?;

            argon2::verify_encoded_ext(hash, check.as_ref(), &[], &[])?
        };

        Ok(result)
    }
}
