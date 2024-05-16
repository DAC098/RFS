use std::path::PathBuf;
use std::sync::{RwLock, RwLockReadGuard};
use std::time::Duration;

use rust_lib_file_sys::wrapper::{
    Encrypted,
    encrypted::{
        Error as WrapperError,
        Key as WrapperKey,
    }
};

use rust_lib_history::versioned::Versioned;
use rust_lib_history::list::fixed::Fixed;
use rfs_lib::sec::chacha;
use rand::RngCore;
use serde::{Serialize, Deserialize};

use crate::net::error::{Error as NetError};

pub const MAX_SESSION_KEYS: usize = 50;

pub type SessionManager = RwLock<Fixed<Key, MAX_SESSION_KEYS>>;
pub type SessionWrapper = Encrypted<SessionManager>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Key {
    data: chacha::Key,
    created: Duration
}

impl Key {
    pub fn rand_key_data() -> Result<chacha::Key, rand::Error> {
        let mut bytes = chacha::empty_key();

        rand::rngs::OsRng.try_fill_bytes(&mut bytes)?;

        Ok(bytes)
    }

    pub fn new(data: chacha::Key, created: Duration) -> Key {
        Key {
            data,
            created
        }
    }

    pub fn data(&self) -> &chacha::Key {
        &self.data
    }

    pub fn data_slice(&self) -> &[u8] {
        &self.data.as_slice()
    }

    pub fn created(&self) -> &Duration {
        &self.created
    }

    pub fn into_tuple(self) -> (chacha::Key, Duration) {
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

impl std::convert::AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.data.as_slice()
    }
}

impl std::convert::AsRef<chacha::Key> for Key {
    fn as_ref(&self) -> &chacha::Key {
        &self.data
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PMError {
    #[error("failed to retrieve reader from rwlock")]
    ReadGuardFailed,

    #[error("failed to retrieve writer from rwlock")]
    WriteGuardFailed,

    #[error(transparent)]
    Wrapper(#[from] WrapperError),
}

impl From<PMError> for NetError {
    fn from(err: PMError) -> Self {
        NetError::new().source(err)
    }
}

#[derive(Debug)]
pub struct PeppersManager(
    Encrypted<RwLock<Versioned<Key>>>
);

impl PeppersManager {
    pub fn load(path: PathBuf, key: WrapperKey) -> Result<Self, PMError> {
        Ok(PeppersManager(
            Encrypted::load_create(path, key)?
        ))
    }

    pub fn reader(&self) -> Result<RwLockReadGuard<'_, Versioned<Key>>, PMError> {
        let Ok(reader) = self.0.inner().read() else {
            return Err(PMError::ReadGuardFailed);
        };

        Ok(reader)
    }

    pub fn get_cb<F, T>(&self, version: &u64, cb: F) -> T
    where
        F: FnOnce(Result<Option<&Key>, PMError>) -> T
    {
        let Ok(reader) = self.0.inner().read() else {
            return cb(Err(PMError::ReadGuardFailed));
        };

        cb(Ok(reader.get(version)))
    }

    pub fn latest_cb<F, T>(&self, cb: F) -> T
    where
        F: FnOnce(Result<Option<(&u64, &Key)>, PMError>) -> T
    {
        let Ok(reader) = self.0.inner().read() else {
            return cb(Err(PMError::ReadGuardFailed));
        };

        cb(Ok(reader.latest_version()))
    }

    pub fn delete(&self, version: &u64) -> Result<(), PMError> {
        {
            let Ok(mut writer) = self.0.inner().write() else {
                return Err(PMError::WriteGuardFailed);
            };

            writer.remove(version);
        }

        Ok(self.0.save()?)
    }

    pub fn update(&self, key: Key) -> Result<(), PMError> {
        {
            let Ok(mut writer) = self.0.inner().write() else {
                return Err(PMError::WriteGuardFailed);
            };

            writer.update(key);
        }

        Ok(self.0.save()?)
    }
}
