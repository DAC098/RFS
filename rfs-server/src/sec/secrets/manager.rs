use std::collections::{BTreeMap, HashMap};
use std::time::{Instant, SystemTime, Duration};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock, PoisonError};
use std::ops::Deref;
use std::sync::{LockResult, RwLockReadGuard};

use rfs_lib::sec::chacha;
use rfs_lib::sec::secrets::Version;
use rfs_lib::sec::secrets::manager::ManagerFile;

use crate::util;
use super::error::{Error, ErrorKind};
use super::fs;
use super::key;

pub type KeyStore = BTreeMap<Version, key::Key>;

pub fn load_manager_file<P>(dir: &P, master_key: &chacha::Key) -> Result<ManagerFile, Error>
where
    P: Deref<Target = Path>
{
    let file_path = dir.join("manager.data");

    if !fs::check_file_exists(file_path.as_path()).map_err(|e| e.with_message("failed locating manager file"))? {
        return Err(Error::new(ErrorKind::FileNotFound).with_message("failed locating manager file"));
    }

    let mut options = std::fs::OpenOptions::new();
    options.read(true);

    let contents = fs::file_to_buffer(file_path, options)
        .map_err(|e| e.with_message("failed reading manager file"))?;

    fs::decrypt(contents, master_key)
        .map_err(|e| e.with_message("failed decrypting manager file"))
}

pub fn save_manager_file<P>(dir: &P, data: &ManagerFile, master_key: &chacha::Key) -> Result<(), Error>
where
    P: Deref<Target = Path>
{
    let file_path = dir.join("manager.data");

    if !fs::check_file_exists(file_path.as_path()).map_err(|e| e.with_message("failed locating manager file"))? {
        return Err(Error::new(ErrorKind::FileNotFound).with_message("failed locating manager file"))?;
    }

    let encrypted = fs::encrypt(data, master_key)
        .map_err(|e| e.with_message("failed encrypting manager file"))?;

    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    options.truncate(true);

    fs::buffer_to_file(file_path, &encrypted, options)
        .map_err(|e| e.with_message("failed writing manager file"))
}

pub fn load_keys<P>(dir: P, master_key: &chacha::Key) -> Result<KeyStore, Error>
where
    P: AsRef<Path>
{
    let reader = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => {
                return Err(Error::new(ErrorKind::DirNotFound));
            },
            _ => {
                return Err(Error::new(ErrorKind::Io).with_source(err))
            }
        }
    };

    let mut store = KeyStore::new();

    for entry in reader {
        let entry = entry.map_err(|err| Error::new(ErrorKind::Io).with_source(err))?;
        let path = entry.path();

        let Some(ext) = path.extension() else {
            return Err(Error::new(ErrorKind::UnknownFile)
                .with_message(format!("unknown file in manager directory: {}", path.display())));
        };
        let Some(stem) = path.file_stem() else {
            return Err(Error::new(ErrorKind::UnknownFile)
                .with_message(format!("unknown file in manager directory: {}", path.display())));
        };

        if stem.eq("manager") && ext.eq("data") {
            continue;
        }

        let Some(utf8_stem) = stem.to_str() else {
            return Err(Error::new(ErrorKind::UnknownFile)
                .with_message(format!("unknown file in manager directory: {}", path.display())));
        };

        let Ok(version) = Version::from_str_radix(utf8_stem, 10) else {
            return Err(Error::new(ErrorKind::UnknownFile)
                .with_message(format!("unknown file in manager directory: {}", path.display())));
        };

        if !ext.eq("key") {
            return Err(Error::new(ErrorKind::UnknownFile)
                .with_message(format!("unknown file in manager directory: {}", path.display())));
        }

        let mut options = std::fs::OpenOptions::new();
        options.read(true);

        let contents = fs::file_to_buffer(path.as_path(), options)
            .map_err(|e| e.with_message("failed reading key file"))?;

        let key = fs::decrypt(contents, master_key)
            .map_err(|e| e.with_message("failed decrypting key file"))?;

        store.insert(version, key);
    }

    Ok(store)
}

#[derive(Debug)]
pub struct Manager {
    store: RwLock<KeyStore>,
    count: Mutex<Version>,
    dir: Box<Path>,
    master_key: chacha::Key,
}

impl Manager {
    pub fn new(dir: PathBuf, master_key: chacha::Key) -> Result<Self, Error> {
        let store = load_keys(&dir, &master_key)?;
        let manager_file = load_manager_file(&dir, &master_key)?;

        let rtn = Manager {
            store: RwLock::new(store),
            count: Mutex::new(manager_file.count),
            dir: dir.into_boxed_path(),
            master_key
        };

        Ok(rtn)
    }

    pub fn create(&mut self, size: usize) -> Result<key::Key, Error> {
        let mut count_guard = self.count.lock()?;
        let version = *count_guard;
        let next_count = *count_guard + 1;

        let new_key = key::Key::generate(version, size)?;

        let mut store_writer = self.store.write()?;

        if store_writer.contains_key(new_key.version()) {
            return Err(Error::new(ErrorKind::VersionExists));
        }

        key::save_key_file(&self.dir, &new_key, &self.master_key)?;

        {
            let to_save = ManagerFile {
                count: next_count
            };

            save_manager_file(&self.dir, &to_save, &self.master_key)?;
        }

        store_writer.insert(version, new_key.clone());

        *count_guard = next_count;

        Ok(new_key)
    }

    pub fn store_ref<'a>(&'a self) -> LockResult<RwLockReadGuard<'a, KeyStore>> {
        self.store.read()
    }

    pub fn get(&self, version: &Version) -> Result<Option<key::Key>, Error> {
        let store_reader = self.store.read()?;

        Ok(store_reader.get(version).cloned())
    }

    pub fn latest(&self) -> Result<key::Key, Error> {
        let store_reader = self.store.read()?;

        if let Some((version, key)) = store_reader.last_key_value() {
            Ok(key.clone())
        } else {
            Ok(key::Key::empty())
        }
    }
}
