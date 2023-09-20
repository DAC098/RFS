use std::path::PathBuf;

use rfs_lib::sec::chacha;
use serde::Deserialize;
use rust_kms_local::fs::Wrapper;

use crate::error;
use crate::config;

use super::secrets;

const BLAKE3_CONTEXT: &str = "rust-file-server 2023-05-12 12:35:00 session tokens";

#[derive(Debug)]
pub enum SessionKey {
    Blake3([u8; 32]),
    HS256([u8; 32]),
    HS384([u8; 32]),
    HS512([u8; 32]),
}

#[derive(Debug)]
pub struct SessionInfo {
    manager: secrets::Manager,
    key: SessionKey,
    domain: Option<String>,
    secure: bool,
}

impl SessionInfo {
    pub fn from_config(config: &config::Config) -> error::Result<Self> {
        let mut session_key = chacha::empty_key();

        config.kdf.expand(
            rfs_lib::sec::secrets::SESSIONS_KEY_INFO,
            &mut session_key
        ).map_err(|e| error::Error::new()
            .kind("KDFExpandFailed")
            .message("failed to expand session key for secrets manager"))?;

        let options = secrets::Options {
            path: config.settings.data.join("sec/secrets/session.data"),
            key: session_key
        };

        let manager = secrets::Manager::load(options)
            .map_err(|e| error::Error::new()
                .kind("SessionManagerFailed")
                .message("failed loading data for session secrets manager")
                .source(e))?;

        let key = SessionKey::Blake3([0; 32]);

        Ok(SessionInfo {
            manager,
            key,
            domain: None,
            secure: config.settings.sec.session.secure
        })
    }

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
    peppers: secrets::Manager,
}

impl Sec {
    pub fn from_config(config: &config::Config) -> error::Result<Sec> {
        let mut password_key = chacha::empty_key();

        config.kdf.expand(
            rfs_lib::sec::secrets::PASSWORDS_KEY_INFO,
            &mut password_key
        ).map_err(|e| error::Error::new()
            .kind("KDFExpandFailed")
            .message("failed to expand passwords key for secrets manager"))?;

        let options = secrets::Options {
            path: config.settings.data.join("sec/secrets/passwords.data"),
            key: password_key
        };

        let peppers = secrets::Manager::load(options)
            .map_err(|e| error::Error::new()
                .kind("PasswordManagerFailed")
                .message("failed loading data for password secrets manager")
                .source(e))?;

        Ok(Sec {
            session_info: SessionInfo::from_config(config)?,
            peppers
        })
    }

    pub fn session_info(&self) -> &SessionInfo {
        &self.session_info
    }

    pub fn secrets(&self) -> &secrets::Manager {
        &self.peppers
    }

    pub fn peppers(&self) -> &secrets::Manager {
        &self.peppers
    }
}

