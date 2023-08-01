use std::fmt::Write;
use futures::TryStreamExt;

use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use rand::RngCore;

use crate::util::{sql, HistoryField};

pub const HASH_LEN: usize = 25;

pub fn create_hash() -> Result<String, rand::Error> {
    let mut bytes = [0u8; HASH_LEN];
    rand::thread_rng().try_fill_bytes(&mut bytes)?;

    Ok(data_encoding::BASE32.encode(&bytes))
}

pub async fn key_exists<K>(
    conn: &impl GenericClient,
    user_id: &ids::UserId,
    key: K
) -> Result<bool, PgError>
where
    K: AsRef<str>
{
    let check = conn.execute(
        "\
        select key \
        from auth_totp \
        where user_id = $1 and \
              key = $2",
        &[user_id, &key.as_ref()]
    ).await?;

    Ok(check == 1)
}

pub struct Hash {
    pub user_id: ids::UserId,
    pub key: HistoryField<String>,
    pub hash: HistoryField<String>,
    pub used: HistoryField<bool>,
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
                used: HistoryField::new(row.get(3)),
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
            &[user_id, &key.as_ref()]
        ).await? {
            Ok(Some(Hash {
                user_id: row.get(0),
                key: HistoryField::new(row.get(1)),
                hash: HistoryField::new(row.get(2)),
                used: HistoryField::new(row.get(3)),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        user_id: &ids::UserId
    ) -> Result<Vec<Self>, PgError> {
        let result = conn.query_raw(
            "\
            select auth_totp_hash.user_id, \
                   auth_totp_hash.key, \
                   auth_totp_hash.hash, \
                   auth_totp_hash.used \
            from auth_totp_hash \
            where auth_totp_hash.user_id = $1",
            [user_id]
        ).await?;

        futures::pin_mut!(result);

        let mut rtn = Vec::with_capacity(3);

        while let Some(row) = result.try_next().await? {
            if rtn.len() == rtn.capacity() {
                rtn.reserve(3);
            }

            rtn.push(Hash {
                user_id: row.get(0),
                key: HistoryField::new(row.get(1)),
                hash: HistoryField::new(row.get(2)),
                used: HistoryField::new(row.get(3))
            });
        }

        rtn.shrink_to_fit();

        Ok(rtn)
    }

    /*
    pub fn user_id(&self) -> &ids::UserId {
        &self.user_id
    }

    pub fn key(&self) -> &str {
        self.key.get_str()
    }

    pub fn hash(&self) -> &str {
        self.hash.get_str()
    }
    */

    pub fn used(&self) -> &bool {
        self.used.get()
    }

    pub fn set_key<K>(&mut self, key: K)
    where
        K: Into<String>
    {
        self.key.set(key.into());
    }

    pub fn set_used(&mut self) -> bool {
        if !self.used.get() {
            self.used.set(true);
            true
        } else {
            false
        }
    }

    pub fn regen_hash(&mut self) -> Result<(), rand::Error> {
        self.hash.set(create_hash()?);
        self.used.set(false);

        Ok(())
    }

    pub fn verify<V>(&self, verify: V) -> bool
    where
        V: AsRef<str>
    {
        self.hash.get_str() == verify.as_ref()
    }

    pub async fn update(&mut self, conn: &impl GenericClient) -> Result<bool, PgError> {
        if !self.key.is_updated() && !self.hash.is_updated() && !self.used.is_updated() {
            return Ok(false);
        }

        let mut update_query = String::from("update auth_totp_hash set");
        let mut update_params: sql::ParamsVec = vec![
            &self.user_id,
            self.key.original()
        ];

        if let Some(new_key) = self.key.updated() {
            write!(
                &mut update_query,
                " key = ${}",
                sql::push_param(&mut update_params, new_key)
            ).unwrap();
        }

        if let Some(new_hash) = self.hash.updated() {
            if update_params.len() > 2 {
                update_query.push(',');
            }

            write!(
                &mut update_query,
                " hash = ${}",
                sql::push_param(&mut update_params, new_hash)
            ).unwrap();
        }

        if let Some(new_used) = self.used.updated() {
            if update_params.len() > 2 {
                update_query.push(',');
            }

            write!(
                &mut update_query,
                " used = ${}",
                sql::push_param(&mut update_params, new_used)
            ).unwrap();
        }

        update_query.push_str(" where user_id = $1 and key = $2");

        let _ = conn.execute(update_query.as_str(), update_params.as_slice()).await?;

        self.key.commit();
        self.hash.commit();
        self.used.commit();

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
