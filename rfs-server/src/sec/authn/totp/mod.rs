use rfs_lib::ids;
use axum::http::StatusCode;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use rand::RngCore;

use crate::net;
use crate::util::HistoryField;

pub mod algo;
pub mod recovery;

pub use algo::Algo;

pub const SECRET_LEN: usize = 25;

pub struct Totp {
    user_id: ids::UserId,
    algo: Algo,
    secret: Vec<u8>,
    digits: u32,
    step: u64,
}

impl Totp {
    fn digits_from_db(v: i32) -> Option<u32> {
        if v < 0 {
            None
        } else {
            Some(v as u32)
        }
    }

    fn step_from_db(v: i32) -> Option<u64> {
        if v < 0 {
            None
        } else {
            Some(v as u64)
        }
    }

    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::UserId,
    ) -> Result<Option<Totp>, PgError> {
        if let Some(row) = conn.query_opt(
            "\
            select auth_totp.algo, \
                   auth_totp.secret, \
                   auth_totp.digits, \
                   auth_totp.step, \
            from auth_totp \
            where auth_totp.user_id = $1",
            &[id]
        ).await? {
            Ok(Some(Totp {
                user_id: id.clone(),
                algo: Algo::from_i16(row.get(1))
                    .expect("unexpected value from database for totp algo"),
                secret: row.get(2),
                digits: Self::digits_from_db(row.get(3))
                    .expect("unexpected value from database for totp digits"),
                step: Self::step_from_db(row.get(4))
                    .expect("unexpected value from database for totp step"),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn algo(&self) -> &Algo {
        &self.algo
    }

    pub fn secret(&self) -> &Vec<u8> {
        &self.secret
    }

    pub fn digits(&self) -> &u32 {
        &self.digits
    }

    pub fn step(&self) -> &u64 {
        &self.step
    }

    pub fn verify<C>(&self, code: C) -> rust_otp::error::Result<rust_otp::VerifyResult>
    where
        C: AsRef<str>
    {
        let settings = rust_otp::TotpSettings {
            algo: self.algo.clone().into(),
            secret: self.secret.clone(),
            digits: self.digits,
            step: self.step,
            window_before: 1,
            window_after: 1,
            now: None,
        };

        rust_otp::verify_totp_code(&settings, code)
    }

    pub async fn delete(&self, conn: &impl GenericClient) -> Result<(), PgError> {
        let _ = conn.execute(
            "delete from auth_totp where user_id = $1",
            &[&self.user_id]
        ).await?;

        Ok(())
    }
}

pub enum TotpHashBuilderError {
    KeyExists,
    Rand(rand::Error),
    Pg(tokio_postgres::Error)
}

impl From<rand::Error> for TotpHashBuilderError {
    fn from(err: rand::Error) -> Self {
        TotpHashBuilderError::Rand(err)
    }
}

impl From<tokio_postgres::Error> for TotpHashBuilderError {
    fn from(err: tokio_postgres::Error) -> Self {
        TotpHashBuilderError::Pg(err)
    }
}

impl From<TotpHashBuilderError> for net::error::Error {
    fn from(err: TotpHashBuilderError) -> Self {
        match err {
            TotpHashBuilderError::KeyExists => net::error::Error::new()
                .status(StatusCode::BAD_REQUEST)
                .kind("TotpHashKeyExists")
                .message("requested totp hash key already exists"),
            TotpHashBuilderError::Rand(err) => err.into(),
            TotpHashBuilderError::Pg(err) => err.into()
        }
    }
}

pub fn create_totp_hash(len: usize) -> Result<String, rand::Error> {
    let mut bytes = Vec::with_capacity(len);

    for _ in 0..bytes.len() {
        bytes.push(0);
    }

    rand::thread_rng().try_fill_bytes(&mut bytes)?;

    Ok(data_encoding::BASE32.encode(&bytes))
}

pub struct TotpHashBuilder {
    user_id: ids::UserId,
    key: String,
    hash_len: Option<usize>,
}

impl TotpHashBuilder {
    pub fn set_hash_len(&mut self, hash_len: usize) {
        self.hash_len = Some(hash_len);
    }

    pub async fn build(self, conn: &impl GenericClient) -> Result<TotpHash, TotpHashBuilderError> {
        let used = false;
        let hash = create_totp_hash(self.hash_len.unwrap_or(25))?;

        let check = conn.execute(
            "\
            select key \
            from auth_totp_hash \
            where key = $1 and \
                  user_id = $2",
            &[&self.key, &self.user_id]
        ).await?;

        if check != 0 {
            return Err(TotpHashBuilderError::KeyExists);
        }

        let _ = conn.execute(
            "\
            insert into auth_totp_hash (user_id, key, hash, used) values
            ($1, $2, $3, $4)",
            &[
                &self.user_id,
                &self.key,
                &hash,
                &used,
            ]
        ).await?;

        Ok(TotpHash {
            user_id: self.user_id,
            key: HistoryField::new(self.key),
            hash: HistoryField::new(hash),
            used,
        })
    }
}

pub struct TotpHash {
    user_id: ids::UserId,
    key: HistoryField<String>,
    hash: HistoryField<String>,
    used: bool,
}

impl TotpHash {
    pub fn builder<K>(user_id: ids::UserId, key: K) -> TotpHashBuilder
    where
        K: Into<String>,
    {
        TotpHashBuilder {
            user_id,
            key: key.into(),
            hash_len: None,
        }
    }

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
            Ok(Some(TotpHash {
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
            Ok(Some(TotpHash {
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
            .map(|row| TotpHash {
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

    pub fn regen_hash(&mut self, len: Option<usize>) -> Result<(), rand::Error> {
        self.hash.set(create_totp_hash(len.unwrap_or(25))?);
        self.used = false;

        Ok(())
    }

    pub fn verify<V>(&self, verify: V) -> bool
    where
        V: AsRef<str>
    {
        self.hash.get() == verify.as_ref()
    }

    pub async fn update(&mut self, conn: &impl GenericClient) -> Result<bool, PgError> {
        if let Some(new_key) = self.key.updated() {
            let check = conn.execute(
                "\
                select key \
                from auth_totp_hash \
                where key = $1 and \
                      user_id = $2",
                &[new_key, &self.user_id]
            ).await?;

            if check != 0 {
                return Ok(false);
            }
        }

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
