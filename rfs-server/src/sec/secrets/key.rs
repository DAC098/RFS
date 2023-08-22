use std::path::Path;

use rfs_lib::sec::secrets::{Version, Timestamp};
use rfs_lib::sec::chacha;
use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;
use rand::RngCore;

use crate::util;
use super::error::{Error, ErrorKind};
use super::fs;

pub fn save_key_file(dir: &Path, data: &Key, master_key: &chacha::Key) -> Result<(), Error> {
    let file_path = dir.join(format!("{}.key", data.version()));

    if fs::check_file_exists(file_path.as_path())? {
        return Err(Error::new(ErrorKind::FileExists).with_message("key file already exists"));
    }

    let encrypted = fs::encrypt(data, master_key)
        .map_err(|e| e.with_message("failed encrypting key file"))?;

    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    options.create_new(true);

    fs::buffer_to_file(file_path.as_path(), &encrypted, options)
        .map_err(|e| e.with_message("failed writing key file"))?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    version: Version,
    bytes: Vec<u8>,
    created: Timestamp,
}

impl Key {
    pub fn new(version: Version, bytes: Vec<u8>, created: Timestamp) -> Key {
        Key {
            version,
            bytes,
            created
        }
    }

    pub fn generate(version: Version, size: usize) -> Result<Self, Error> {
        let created = util::utc_now().ok_or(Error::new(ErrorKind::Timestamp))?;
        let mut bytes = Vec::with_capacity(size);

        rand::thread_rng()
            .try_fill_bytes(bytes.as_mut_slice())
            .map_err(|err| Error::new(ErrorKind::Rand).with_source(err))?;

        Ok(Key {
            version,
            bytes,
            created,
        })
    }

    pub fn empty() -> Key {
        Key {
            version: 0,
            bytes: Vec::new(),
            created: 0,
        }
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn created(&self) -> &Timestamp {
        &self.created
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes.as_slice()
    }
}

