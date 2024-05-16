use chacha20poly1305::{
    aead::{Aead, AeadCore, OsRng},
    XChaCha20Poly1305,
    KeyInit,
    XNonce,
};

pub const NONCE_LEN: usize = 24;
pub const KEY_LEN: usize = 32;

pub type Key = [u8; KEY_LEN];

#[inline]
pub fn empty_key() -> Key {
    [0; KEY_LEN]
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("invalid data provided")]
    InvalidData,

    #[error("data encryption failed")]
    EncryptFailed,

    #[error("data decryption failed")]
    DecryptFailed,
}

fn decode_data(mut data: Vec<u8>) -> Result<(XNonce, Vec<u8>), CryptoError> {
    if data.len() < NONCE_LEN {
        return Err(CryptoError::InvalidData);
    }

    let nonce = XNonce::from_exact_iter(data.drain(..NONCE_LEN)).unwrap();

    Ok((nonce, data))
}

fn encode_data(nonce: XNonce, data: Vec<u8>) -> Vec<u8> {
    let mut rtn: Vec<u8> = Vec::with_capacity(nonce.len() + data.len());
    rtn.extend(nonce);
    rtn.extend(data);

    rtn
}

pub fn decrypt_data(key: &Key, data: Vec<u8>) -> Result<Vec<u8>, CryptoError> {
    let (nonce, encrypted) = decode_data(data)?;

    let cipher = XChaCha20Poly1305::new_from_slice(key).unwrap();

    let Ok(result) = cipher.decrypt(&nonce, encrypted.as_slice()) else {
        return Err(CryptoError::DecryptFailed);
    };

    Ok(result)
}

pub fn encrypt_data(key: &Key, data: Vec<u8>) -> Result<Vec<u8>, CryptoError> {
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let cipher = XChaCha20Poly1305::new_from_slice(key).unwrap();

    let Ok(encrypted) = cipher.encrypt(&nonce, data.as_slice()) else {
        return Err(CryptoError::EncryptFailed);
    };

    Ok(encode_data(nonce, encrypted))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encrypt_decrypt() {
        let bytes = b"i am test data to encrypt and decrypt";
        let empty_key = empty_key();

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
