use rand::RngCore;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;

use crate::net::error;

pub const SESSION_ID_BYTES: usize = 48;

#[derive(Debug, thiserror::Error)]
pub enum UniqueError {
    #[error(transparent)]
    Rand(#[from] rand::Error),

    #[error(transparent)]
    Pg(#[from] PgError),
}

impl From<UniqueError> for error::Error {
    fn from(err: UniqueError) -> error::Error {
        match err {
            UniqueError::Rand(e) => e.into(),
            UniqueError::Pg(e) => e.into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SessionToken([u8; SESSION_ID_BYTES]);

impl SessionToken {
    pub fn from_vec(vec: Vec<u8>) -> Self {
        TryFrom::try_from(vec)
            .expect("invalid vector length for session token")
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

    pub async fn unique(conn: &impl GenericClient, mut attempts: usize) -> Result<Option<Self>, UniqueError> {
        let mut rtn = [0; SESSION_ID_BYTES];
        let mut count;

        while attempts > 0 {
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

            attempts -= 1;
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

#[derive(Debug, thiserror::Error)]
#[error("data does not have the proper length")]
pub struct InvalidLength;

impl TryFrom<Vec<u8>> for SessionToken {
    type Error = InvalidLength;

    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        if let Ok(array) = vec.try_into() {
            Ok(SessionToken(array))
        } else {
            Err(InvalidLength)
        }
    }
}
