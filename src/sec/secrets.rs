use std::sync::RwLock;
use std::time::Duration;

use rust_lib_file_sys::wrapper::Encrypted;

use rust_lib_history::versioned::Versioned;
use rust_lib_history::list::fixed::Fixed;
use rand::RngCore;
use serde::{Serialize, Deserialize};

pub const KEY_DATA_LEN: usize = 32;
pub const MAX_SESSION_KEYS: usize = 50;

pub type KeyData = [u8; KEY_DATA_LEN];
pub type PepperManager = RwLock<Versioned<Key>>;
pub type SessionManager = RwLock<Fixed<Key, MAX_SESSION_KEYS>>;
pub type PepperWrapper = Encrypted<PepperManager>;
pub type SessionWrapper = Encrypted<SessionManager>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Key {
    data: KeyData,
    created: Duration
}

impl Key {
    pub fn rand_key_data() -> Result<KeyData, rand::Error> {
        let mut bytes = [0; KEY_DATA_LEN];

        rand::rngs::OsRng.try_fill_bytes(&mut bytes)?;

        Ok(bytes)
    }

    pub fn new(data: KeyData, created: Duration) -> Key {
        Key {
            data,
            created
        }
    }

    pub fn data(&self) -> &KeyData {
        &self.data
    }

    pub fn created(&self) -> &Duration {
        &self.created
    }

    pub fn into_tuple(self) -> (KeyData, Duration) {
        (self.data, self.created)
    }
}

impl Clone for Key {
    fn clone(&self) -> Self {
        Key {
            data: self.data.clone(),
            created: self.created.clone(),
        }
    }
}

impl Copy for Key {}

impl std::convert::AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.data.as_slice()
    }
}

impl std::convert::AsRef<[u8; KEY_DATA_LEN]> for Key {
    fn as_ref(&self) -> &[u8; KEY_DATA_LEN] {
        &self.data
    }
}
