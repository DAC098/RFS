use lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;

pub mod secret;
pub mod state;

pub mod password;
pub mod totp;

pub mod session;

pub mod initiator;

pub enum Verify {
    Totp(totp::Totp)
}

impl Verify {
    pub async fn retrieve_primary(
        conn: &impl GenericClient,
        id: &ids::UserId,
    ) -> Result<Option<Verify>, PgError> {
        if let Some(totp) = totp::Totp::retrieve(conn, id).await? {
            Ok(Some(Verify::Totp(totp)))
        } else {
            Ok(None)
        }
    }
}

pub enum Authorize {
    Password(password::Password)
}

impl Authorize {
    pub async fn retrieve_primary(
        conn: &impl GenericClient,
        id: &ids::UserId,
    ) -> Result<Option<Authorize>, PgError> {
        if let Some(password) = password::Password::retrieve(conn, id).await? {
            Ok(Some(Authorize::Password(password)))
        } else {
            Ok(None)
        }
    }
}
