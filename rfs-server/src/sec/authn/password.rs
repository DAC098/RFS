use rfs_lib::ids;
use rfs_lib::history::HistoryField;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use argon2::Variant;
use rand::RngCore;

use crate::net;
use crate::sec::secrets;

pub const SALT_LEN: usize = 32;
pub type SaltType = [u8; SALT_LEN];

pub fn gen_salt() -> Result<SaltType, rand::Error> {
    let mut salt = [0u8; SALT_LEN];

    rand::thread_rng().try_fill_bytes(salt.as_mut_slice())?;

    Ok(salt)
}

pub fn gen_hash(password: &str, salt: &[u8], secret: &[u8]) -> Result<String, argon2::Error> {
    let mut config = argon2::Config::default();
    config.mem_cost = 19456;
    config.variant = Variant::Argon2id;
    config.secret = secret;

    let hash = argon2::hash_encoded(
        password.as_bytes(),
        salt,
        &config
    )?;

    Ok(hash)
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
            let version: i64 = row.get(1);

            Ok(Some(Password {
                user_id: row.get(0),
                version: version.try_into()
                    .expect("auth_password.version is an invalid unsigned integer"),
                hash: row.get(2)
            }))
        } else {
            Ok(None)
        }
    }

    pub fn verify<C>(&self, check: C, secret: &[u8]) -> Result<bool, argon2::Error>
    where
        C: AsRef<[u8]>
    {
        let ad = [0u8; 0];

        let result = argon2::verify_encoded_ext(
            &self.hash.as_str(),
            check.as_ref(),
            secret,
            &ad
        )?;

        Ok(result)
    }
}
