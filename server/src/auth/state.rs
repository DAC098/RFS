use crate::error;
use crate::auth::secret;

const BLAKE3_CONTEXT: &str = "rust-file-server 2023-05-12 12:35:00 session tokens";

#[derive(clap::ValueEnum, Clone, Debug)]
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
pub struct Builder {
    session_hash: Option<SessionHash>,
    session_secret: Option<String>,
    session_domain: Option<String>,
    session_secure: bool,
    secret_manager: secret::Manager,
}

impl Builder {
    pub fn set_session_secret(&mut self, key: String) -> &mut Self {
        self.session_secret = Some(key);
        self
    }

    pub fn with_session_secret(mut self, key: String) -> Self {
        self.session_secret = Some(key);
        self
    }

    pub fn set_session_hash(&mut self, hash: SessionHash) -> &mut Self {
        self.session_hash = Some(hash);
        self
    }

    pub fn with_session_hash(mut self, hash: SessionHash) -> Self {
        self.session_hash = Some(hash);
        self
    }

    pub fn set_session_secure(&mut self, secure: bool) -> &mut Self {
        self.session_secure = secure;
        self
    }

    pub fn with_session_secure(mut self, secure: bool) -> Self {
        self.session_secure = secure;
        self
    }

    pub fn add_secret(&mut self, version: u32, bytes: Vec<u8>) -> bool {
        self.secret_manager.add(secret::Secret::new(version, bytes))
    }

    pub fn build(self) -> error::Result<Auth> {
        let session_secret = self.session_secret.unwrap_or(String::from("secret"));

        let session_key = match self.session_hash.unwrap_or(SessionHash::Blake3) {
            SessionHash::Blake3 => {

                SessionKey::Blake3(blake3::derive_key(
                    BLAKE3_CONTEXT,
                    session_secret.as_bytes()
                ))
            },
            SessionHash::HS256 => {
                let hk = hkdf::Hkdf::<sha3::Sha3_256>::new(None, session_secret.as_bytes());
                let mut bytes = [0u8; 64];

                hk.expand(&[], &mut bytes)?;

                SessionKey::HS256(bytes)
            },
            SessionHash::HS384 => {
                let hk = hkdf::Hkdf::<sha3::Sha3_384>::new(None, session_secret.as_bytes());
                let mut bytes = [0u8; 64];

                hk.expand(&[], &mut bytes)?;

                SessionKey::HS384(bytes)
            },
            SessionHash::HS512 => {
                let hk = hkdf::Hkdf::<sha3::Sha3_512>::new(None, session_secret.as_bytes());
                let mut bytes = [0u8; 64];

                hk.expand(&[], &mut bytes)?;

                SessionKey::HS512(bytes)
            }
        };

        Ok(Auth {
            session_info: SessionInfo {
                key: session_key,
                domain: self.session_domain,
                secure: self.session_secure,
            },
            secrets: self.secret_manager,
        })
    }
}

#[derive(Debug)]
pub enum SessionKey {
    Blake3([u8; 32]),
    HS256([u8; 64]),
    HS384([u8; 64]),
    HS512([u8; 64]),
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
pub struct Auth {
    session_info: SessionInfo,
    secrets: secret::Manager,
}

impl Auth {
    pub fn builder() -> Builder {
        Builder {
            session_hash: None,
            session_secret: None,
            session_domain: None,
            session_secure: false,
            secret_manager: secret::Manager::new(),
        }
    }

    pub fn session_info(&self) -> &SessionInfo {
        &self.session_info
    }

    pub fn secrets(&self) -> &secret::Manager {
        &self.secrets
    }
}
