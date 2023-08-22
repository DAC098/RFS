use std::path::Path;
use std::io::{Write, Read};

use rfs_lib::sec::chacha;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

use super::error::{Error, ErrorKind};

pub fn decrypt<'a, T>(buffer: Vec<u8>, master_key: &chacha::Key) -> Result<T, Error>
where
    T: DeserializeOwned
{
    let decrypted = match chacha::decrypt_data(master_key, buffer) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(match err {
                chacha::Error::ChaCha => Error::new(ErrorKind::DecryptFailed),
                chacha::Error::InvalidEncoding => Error::new(ErrorKind::InvalidEncoding),
                chacha::Error::Rand(_) => unreachable!(),
            })
        }
    };

    let deserialized = bincode::deserialize(decrypted.as_slice())
        .map_err(|err| Error::new(ErrorKind::DeserializeFailed)
            .with_source(err))?;

    Ok(deserialized)
}

pub fn encrypt<T>(data: &T, master_key: &chacha::Key) -> Result<Vec<u8>, Error>
where
    T: Serialize
{
    let serialized = bincode::serialize(data)
        .map_err(|err| Error::new(ErrorKind::SerializeFailed)
            .with_source(err))?;

    let encrypted = match chacha::encrypt_data(master_key, serialized) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(match err {
                chacha::Error::ChaCha => Error::new(ErrorKind::EncryptFailed),
                chacha::Error::Rand(inner) => Error::new(ErrorKind::Rand).with_source(inner),
                chacha::Error::InvalidEncoding => unreachable!(),
            })
        }
    };

    Ok(encrypted)
}

pub fn check_file_exists<P>(path: P) -> Result<bool, Error>
where
    P: AsRef<Path>
{
    let file_path = path.as_ref();

    let metadata = match file_path.metadata() {
        Ok(m) => m,
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => {
                return Ok(false);
            },
            _ => {
                return Err(Error::new(ErrorKind::Io).with_source(err));
            }
        }
    };

    if !metadata.is_file() {
        return Err(Error::new(ErrorKind::NotAFile));
    }

    Ok(true)
}

pub fn file_to_buffer<P>(file_path: P, options: std::fs::OpenOptions) -> Result<Vec<u8>, Error>
where
    P: AsRef<Path>
{
    let file = options.open(file_path)
        .map_err(|err| Error::new(ErrorKind::Io).with_source(err))?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = Vec::new();

    reader.read_to_end(&mut buffer)
        .map_err(|err| Error::new(ErrorKind::Io).with_source(err))?;

    Ok(buffer)
}

pub fn buffer_to_file<P, B>(file_path: P, buffer: B, options: std::fs::OpenOptions) -> Result<(), Error>
where
    P: AsRef<Path>,
    B: AsRef<[u8]>
{
    let file = options.open(file_path)
        .map_err(|err| Error::new(ErrorKind::Io).with_source(err))?;
    let mut writer = std::io::BufWriter::new(file);

    writer.write_all(buffer.as_ref())
        .map_err(|err| Error::new(ErrorKind::Io).with_source(err))?;

    Ok(())
}

