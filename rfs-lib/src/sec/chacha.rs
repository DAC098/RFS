use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::Aead,
    KeyInit,
    Error as ChaChaError
};
use rand::RngCore;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 24;

pub type Key = [u8; KEY_LEN];
pub type Nonce = [u8; NONCE_LEN];

#[derive(Debug)]
pub enum Error {
    InvalidEncoding,
    ChaCha,
    Rand(rand::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Rand(e) => write!(f, "Error::Rand {}", e),
            Error::ChaCha => write!(f, "Error::ChaCha"),
            Error::InvalidEncoding => write!(f, "Error::InvalidEncoding")
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Rand(e) => Some(e),
            Error::ChaCha |
            Error::InvalidEncoding => None
        }
    }
}

impl From<rand::Error> for Error {
    fn from(e: rand::Error) -> Self {
        Error::Rand(e)
    }
}

impl From<ChaChaError> for Error {
    fn from(_: ChaChaError) -> Self {
        Error::ChaCha
    }
}

#[inline]
pub fn empty_key() -> Key {
    [0; KEY_LEN]
}

pub fn make_nonce() -> Result<Nonce, Error> {
    let mut nonce: Nonce = [0; NONCE_LEN];

    rand::rngs::OsRng.try_fill_bytes(&mut nonce)?;

    Ok(nonce)
}

fn decode_data(data: Vec<u8>) -> Result<(Nonce, Vec<u8>), Error> {
    let mut nonce: Nonce = [0; NONCE_LEN];
    let mut encrypted: Vec<u8> = Vec::with_capacity(data.len() - nonce.len());
    let mut iter = data.into_iter();

    for i in 0..nonce.len() {
        if let Some(b) = iter.next() {
            nonce[i] = b;
        } else {
            return Err(Error::InvalidEncoding);
        }
    }

    while let Some(b) = iter.next() {
        encrypted.push(b);
    }

    Ok((nonce, encrypted))
}

fn encode_data(nonce: Nonce, data: Vec<u8>) -> Result<Vec<u8>, Error> {
    let mut rtn: Vec<u8> = Vec::with_capacity(nonce.len() + data.len());

    for b in nonce {
        rtn.push(b);
    }

    for b in data {
        rtn.push(b);
    }

    Ok(rtn)
}

pub fn decrypt_data(key: &Key, data: Vec<u8>) -> Result<Vec<u8>, Error> {
    let (nonce, encrypted) = decode_data(data)?;

    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .expect("invalid key provided to chacha cipher");

    Ok(cipher.decrypt((&nonce).into(), encrypted.as_slice())?)
}

pub fn encrypt_data(key: &Key, data: Vec<u8>) -> Result<Vec<u8>, Error> {
    let nonce = make_nonce()?;
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .expect("invalid key provded to chacha cipher");

    let encrypted = cipher.encrypt((&nonce).into(), data.as_slice())?;

    encode_data(nonce, encrypted)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encrypt_decrypt() {
        let bytes = b"i am test data to encrypt and decrypt";
        let empty_key = [0u8; KEY_LEN];

        let encrypted = match encrypt_data(&empty_key, bytes.to_vec()) {
            Ok(e) => e,
            Err(err) => {
                panic!("failed to encrypt data: {}\nbytes: {:?}", err, bytes);
            }
        };

        let decrypted = match decrypt_data(&empty_key, encrypted.clone()) {
            Ok(d) => d,
            Err(err) => {
                if let Ok((nonce, data)) = decode_data(encrypted.clone()) {
                    panic!("failed to decrypt data: {}\nnonce: {:?}\ndata: {:?}", err, nonce, data);
                } else {
                    panic!("failed to decrypt data: {}\nencrypted: {:?}", err, encrypted);
                }
            }
        };

        assert_eq!(bytes, decrypted.as_slice());
    }
}
