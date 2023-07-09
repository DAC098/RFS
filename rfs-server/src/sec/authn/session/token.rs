use rand::RngCore;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use hmac::Mac;

use crate::net::error;

pub const SESSION_ID_BYTES: usize = 48;

pub enum UniqueError {
    Rand(rand::Error),
    Pg(PgError),
}

impl From<PgError> for UniqueError {
    fn from(err: PgError) -> Self {
        UniqueError::Pg(err)
    }
}

impl From<rand::Error> for UniqueError {
    fn from(err: rand::Error) -> Self {
        UniqueError::Rand(err)
    }
}

impl From<UniqueError> for error::Error {
    fn from(err: UniqueError) -> error::Error {
        match err {
            UniqueError::Rand(e) => e.into(),
            UniqueError::Pg(e) => e.into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SessionToken([u8; SESSION_ID_BYTES]);

impl SessionToken {
    pub fn new() -> Result<Self, rand::Error> {
        let mut array = [0; SESSION_ID_BYTES];

        rand::thread_rng().try_fill_bytes(&mut array)?;

        Ok(SessionToken(array))
    }

    pub fn from_array(array: [u8; SESSION_ID_BYTES]) -> Self {
        SessionToken(array)
    }

    pub fn from_vec(mut vec: Vec<u8>) -> Self {
        let mut array = [0; SESSION_ID_BYTES];
        let mut index = 0;

        for v in vec.drain(0..SESSION_ID_BYTES) {
            array[index] = v;
            index += 1;
        }

        SessionToken(array)
    }

    pub fn drain_vec(vec: &mut Vec<u8>) -> Self {
        let mut array = [0; SESSION_ID_BYTES];
        let mut index = 0;

        for v in vec.drain(0..SESSION_ID_BYTES) {
            array[index] = v;
            index += 1;
        }

        SessionToken(array)
    }


    pub async fn unique(conn: &impl GenericClient) -> Result<Option<Self>, UniqueError> {
        let mut rtn = [0; SESSION_ID_BYTES];
        let mut count = 0;

        loop {
            rand::thread_rng().try_fill_bytes(&mut rtn)?;

            count = conn.execute(
                "select token from auth_session where token = $1",
                &[&rtn.as_slice()]
            ).await?;

            if count == 0 {
                return Ok(Some(SessionToken(rtn)));
            } else {
                rtn.fill(0);
            }
        }

        Ok(None)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl AsRef<[u8]> for SessionToken {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for SessionToken {
    fn from(vec: Vec<u8>) -> Self {
        Self::from_vec(vec)
    }
}

pub async fn exists_check<T>(conn: &impl GenericClient, token: T) -> Result<bool, PgError>
where
    T: AsRef<[u8]>
{
    let count = conn.execute(
        "select token from auth_session where token = $1",
        &[&token.as_ref()]
    ).await?;

    Ok(count != 0)
}
