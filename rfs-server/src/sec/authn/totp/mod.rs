use std::fmt::Write;

use rfs_lib::ids;
use rfs_lib::history::HistoryField;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use rand::RngCore;

use crate::sql;

pub mod algo;
pub mod recovery;

pub use algo::Algo;

pub const SECRET_LEN: usize = 25;

pub fn create_secret() -> Result<Vec<u8>, rand::Error> {
    let mut bytes = [0u8; SECRET_LEN];
    rand::thread_rng().try_fill_bytes(&mut bytes)?;

    Ok(bytes.to_vec())
}

pub struct Totp {
    pub user_id: ids::UserId,
    pub algo: HistoryField<Algo>,
    pub secret: HistoryField<Vec<u8>>,
    pub digits: HistoryField<u32>,
    pub step: HistoryField<u64>,
}

impl Totp {
    fn digits_from_db(v: i32) -> u32 {
        v as u32
    }

    fn step_from_db(v: i32) -> u64 {
        v as u64
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
                   auth_totp.step \
            from auth_totp \
            where auth_totp.user_id = $1",
            &[id]
        ).await? {
            Ok(Some(Totp {
                user_id: id.clone(),
                algo: HistoryField::new(Algo::from_i16(row.get(0)).unwrap()),
                secret: HistoryField::new(row.get(1)),
                digits: HistoryField::new(Self::digits_from_db(row.get(2))),
                step: HistoryField::new(Self::step_from_db(row.get(3)))
            }))
        } else {
            Ok(None)
        }
    }

    /*
    pub fn algo(&self) -> &Algo {
        self.algo.get()
    }

    pub fn secret(&self) -> &Vec<u8> {
        self.secret.get()
    }
    */

    pub fn digits(&self) -> &u32 {
        self.digits.get()
    }

    /*
    pub fn step(&self) -> &u64 {
        self.step.get()
    }
    */

    pub fn set_algo(&mut self, algo: Algo) {
        self.algo.set(algo);
    }

    pub fn set_digits(&mut self, digits: u32) {
        self.digits.set(digits);
    }

    pub fn set_step(&mut self, step: u64) {
        self.step.set(step);
    }

    pub fn regen_secret(&mut self) -> Result<(), rand::Error> {
        self.secret.set(create_secret()?);

        Ok(())
    }

    pub fn verify<C>(&self, code: C) -> rust_otp::error::Result<rust_otp::VerifyResult>
    where
        C: AsRef<str>
    {
        let algo = self.algo.get().clone().into();
        let secret = self.secret.get().clone();

        let settings = rust_otp::TotpSettings {
            algo,
            secret,
            digits: *self.digits.get(),
            step: *self.step.get(),
            window_before: 1,
            window_after: 1,
            now: None,
        };

        rust_otp::verify_totp_code(&settings, code)
    }

    pub async fn update(&mut self, conn: &impl GenericClient) -> Result<bool, PgError> {
        if !self.algo.is_updated() && !self.secret.is_updated() && !self.digits.is_updated() && !self.step.is_updated() {
            return Ok(false);
        }

        let pg_algo;
        let pg_digits;
        let pg_step;
        let mut update_query = String::from("update auth_totp set");
        let mut update_params: sql::ParamsVec = vec![
            &self.user_id
        ];

        if let Some(new_algo) = self.algo.updated() {
            pg_algo = new_algo.as_i16();

            write!(
                &mut update_query,
                " algo = ${}",
                sql::push_param(&mut update_params, &pg_algo)
            ).unwrap();
        }

        if let Some(new_secret) = self.secret.updated() {
            if update_params.len() > 1 {
                update_query.push(',');
            }

            write!(
                &mut update_query,
                " secret = ${}",
                sql::push_param(&mut update_params, new_secret)
            ).unwrap();
        }

        if let Some(new_digits) = self.digits.updated() {
            if update_params.len() > 1 {
                update_query.push(',');
            }

            pg_digits = *new_digits as i32;

            write!(
                &mut update_query,
                " digits = ${}",
                sql::push_param(&mut update_params, &pg_digits)
            ).unwrap();
        }

        if let Some(new_step) = self.step.updated() {
            if update_params.len() > 1 {
                update_query.push(',');
            }

            pg_step = *new_step as i32;

            write!(
                &mut update_query,
                " step = ${}",
                sql::push_param(&mut update_params, &pg_step)
            ).unwrap();
        }

        update_query.push_str(" where user_id = $1");

        conn.execute(update_query.as_str(), update_params.as_slice()).await?;

        self.algo.commit();
        self.secret.commit();
        self.digits.commit();
        self.step.commit();

        Ok(true)
    }

    pub async fn delete(&self, conn: &impl GenericClient) -> Result<(), PgError> {
        let _ = conn.execute(
            "delete from auth_totp where user_id = $1",
            &[&self.user_id]
        ).await?;

        Ok(())
    }
}
