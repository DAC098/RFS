use lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use rand::RngCore;

use super::totp;

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

