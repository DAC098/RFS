use serde::Deserialize;

use crate::error;

use super::secrets;

const BLAKE3_CONTEXT: &str = "rust-file-server 2023-05-12 12:35:00 session tokens";

#[derive(clap::ValueEnum, Clone, Debug, Deserialize)]
pub enum SessionHash {
    Blake3,
    /// hmac SHA3-256
    HS256,
    /// hmac SHA3-384
    HS384,
    /// hmac SHA3-512
    HS512,
}

#[derive(Debug)]
pub struct SessionInfoBuilder {
    hash: Option<SessionHash>,
    secret: Option<String>,
    domain: Option<String>,
    secure: bool
}

impl SessionInfoBuilder {
    pub fn set_secret(&mut self, key: String) -> &mut Self {
        self.secret = Some(key);
        self
    }

    pub fn set_hash(&mut self, hash: SessionHash) -> &mut Self {
        self.hash = Some(hash);
        self
    }

    pub fn set_secure(&mut self, secure: bool) -> &mut Self {
        self.secure = secure;
        self
    }

    pub fn build(self) -> error::Result<SessionInfo> {
        let secret = self.secret.unwrap_or(String::from("secret"));
        let domain = self.domain;
        let secure = self.secure;

        let key = match self.hash.unwrap_or(SessionHash::Blake3) {
            SessionHash::Blake3 => {
                SessionKey::Blake3(blake3::derive_key(
                    BLAKE3_CONTEXT,
                    secret.as_bytes()
                ))
            },
            SessionHash::HS256 => {
                let hk = hkdf::Hkdf::<sha3::Sha3_256>::new(None, secret.as_bytes());
                let mut bytes = [0u8; 32];

                hk.expand(&[], &mut bytes)?;

                SessionKey::HS256(bytes)
            },
            SessionHash::HS384 => {
                let hk = hkdf::Hkdf::<sha3::Sha3_384>::new(None, secret.as_bytes());
                let mut bytes = [0u8; 32];

                hk.expand(&[], &mut bytes)?;

                SessionKey::HS384(bytes)
            },
            SessionHash::HS512 => {
                let hk = hkdf::Hkdf::<sha3::Sha3_512>::new(None, secret.as_bytes());
                let mut bytes = [0u8; 32];

                hk.expand(&[], &mut bytes)?;

                SessionKey::HS512(bytes)
            }
        };

        Ok(SessionInfo {
            key,
            domain,
            secure,
        })
    }
}

#[derive(Debug)]
pub struct Builder {
    session_info: SessionInfoBuilder,
    secret_manager: secrets::Manager,
}

impl Builder {
    pub fn session_info(&mut self) -> &mut SessionInfoBuilder {
        &mut self.session_info
    }

    pub fn build(self) -> error::Result<Sec> {
        Ok(Sec {
            session_info: self.session_info.build()?,
            secrets: self.secret_manager,
        })
    }
}

#[derive(Debug)]
pub enum SessionKey {
    Blake3([u8; 32]),
    HS256([u8; 32]),
    HS384([u8; 32]),
    HS512([u8; 32]),
}

#[derive(Debug)]
pub struct SessionInfo {
    key: SessionKey,
    domain: Option<String>,
    secure: bool,
}

impl SessionInfo {
    pub fn key(&self) -> &SessionKey {
        &self.key
    }

    pub fn domain(&self) -> Option<&String> {
        self.domain.as_ref()
    }

    pub fn secure(&self) -> &bool {
        &self.secure
    }
}

#[derive(Debug)]
pub struct Sec {
    session_info: SessionInfo,
    secrets: secrets::Manager,
}

impl Sec {
    pub fn builder() -> Builder {
        Builder {
            session_info: SessionInfoBuilder {
                hash: None,
                secret: None,
                domain: None,
                secure: false,
            },
            secret_manager: secrets::Manager::new(),
        }
    }

    pub fn session_info(&self) -> &SessionInfo {
        &self.session_info
    }

    pub fn secrets(&self) -> &secrets::Manager {
        &self.secrets
    }
}
