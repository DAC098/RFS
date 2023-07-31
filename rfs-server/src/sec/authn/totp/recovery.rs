use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use rand::RngCore;

use crate::util::HistoryField;

pub const HASH_LEN: usize = 25;

pub fn create_hash() -> Result<String, rand::Error> {
    let mut bytes = [0u8; 25];
    rand::thread_rng().try_fill_bytes(&mut bytes)?;

    Ok(data_encoding::BASE32.encode(&bytes))
}

pub struct Hash {
    user_id: ids::UserId,
    key: HistoryField<String>,
    hash: HistoryField<String>,
    used: bool,
}

impl Hash {
    pub async fn retrieve_hash<H>(
        conn: &impl GenericClient,
        user_id: &ids::UserId,
        hash: H,
    ) -> Result<Option<Self>, PgError> 
    where
        H: AsRef<str>
    {
        if let Some(row) = conn.query_opt(
            "\
            select auth_totp_hash.user_id, \
                   auth_totp_hash.key, \
                   auth_totp_hash.hash, \
                   auth_totp_hash.used \
            from auth_totp_hash \
            where auth_totp_hash.user_id = $1 and
                  auth_totp_hash.hash = $2",
            &[
                user_id,
                &hash.as_ref()
            ]
        ).await? {
            Ok(Some(Hash {
                user_id: row.get(0),
                key: HistoryField::new(row.get(1)),
                hash: HistoryField::new(row.get(2)),
                used: row.get(3),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn retrieve_key<K>(
        conn: &impl GenericClient,
        user_id: &ids::UserId,
        key: K
    ) -> Result<Option<Self>, PgError> 
    where
        K: AsRef<str>
    {
        if let Some(row) = conn.query_opt(
            "\
            select auth_totp_hash.user_id, \
                   auth_totp_hash.key, \
                   auth_totp_hash.hash, \
                   auth_totp_hash.used \
            from auth_totp_hash \
            where auth_totp_hash.user_id = $1 and \
                  auth_totp_hash.key = $2",
            &[
                user_id,
                &key.as_ref()
            ]
        ).await? {
            Ok(Some(Hash {
                user_id: row.get(0),
                key: HistoryField::new(row.get(1)),
                hash: HistoryField::new(row.get(2)),
                used: row.get(3),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        user_id: &ids::UserId
    ) -> Result<Vec<Self>, PgError> {
        let result = conn.query(
            "\
            select auth_totp_hash.user_id, \
                   auth_totp_hash.key, \
                   auth_totp_hash.hash, \
                   auth_totp_hash.used \
            from auth_totp_hash \
            where auth_totp_hash.user_id = $1",
            &[user_id]
        )
            .await?
            .into_iter()
            .map(|row| Hash {
                user_id: row.get(0),
                key: HistoryField::new(row.get(1)),
                hash: HistoryField::new(row.get(2)),
                used: row.get(3)
            })
            .collect();

        Ok(result)
    }

    pub fn user_id(&self) -> &ids::UserId {
        &self.user_id
    }

    pub fn key(&self) -> &str {
        self.key.get_str()
    }

    pub fn hash(&self) -> &str {
        self.hash.get_str()
    }

    pub fn used(&self) -> &bool {
        &self.used
    }

    pub fn set_key<K>(&mut self, key: K)
    where
        K: Into<String>
    {
        self.key.set(key.into());
    }

    pub fn set_used(&mut self) -> bool {
        if !self.used {
            self.used = true;
            true
        } else {
            false
        }
    }

    pub fn regen_hash(&mut self) -> Result<(), rand::Error> {
        self.hash.set(create_hash()?);
        self.used = false;

        Ok(())
    }

    pub fn verify<V>(&self, verify: V) -> bool
    where
        V: AsRef<str>
    {
        self.hash.get_str() == verify.as_ref()
    }

    pub async fn update(&mut self, conn: &impl GenericClient) -> Result<bool, PgError> {
        let _ = conn.execute(
            "\
            update auth_totp_hash \
            set key = $3, \
                used = $4, \
                hash = $5 \
            where key = $1 and user_id = $2",
            &[
                &self.key.original(),
                &self.user_id,
                &self.key.get(),
                &self.used,
                &self.hash.get()
            ]
        ).await?;

        self.key.commit();
        self.hash.commit();

        Ok(true)
    }

    pub async fn delete(&self, conn: &impl GenericClient) -> Result<(), PgError> {
        conn.execute(
            "delete from auth_totp_hash where key = $1 and user_id = $2",
            &[&self.key.original(), &self.user_id]
        ).await?;

        Ok(())
    }
}
