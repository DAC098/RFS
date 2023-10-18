use std::path::PathBuf;

use rfs_lib::sec::chacha;
use serde::Deserialize;
use rust_kms_local::fs::Wrapper;

use crate::error;
use crate::config;

use super::secrets;

#[derive(Debug)]
pub struct SessionInfo {
    manager: secrets::SessionManager,
    domain: Option<String>,
    secure: bool,
}

impl SessionInfo {
    pub fn from_config(config: &config::Config) -> error::Result<Self> {
        let mut session_key = chacha::empty_key();

        config.kdf.expand(
            rfs_lib::sec::secrets::SESSIONS_KEY_INFO,
            &mut session_key
        ).map_err(|_| error::Error::new()
            .kind("KDFExpandFailed")
            .message("failed to expand session key for secrets manager"))?;

        let manager = secrets::SessionManager::load(
            config.settings.data.join("sec/secrets/session.data"),
            session_key
        ).map_err(|e| error::Error::new()
            .kind("SessionManagerFailed")
            .message("failed loading data for session secrets manager")
            .source(e))?;

        Ok(SessionInfo {
            manager,
            domain: None,
            secure: config.settings.sec.session.secure
        })
    }

    pub fn keys(&self) -> &secrets::SessionManager {
        &self.manager
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
    peppers: secrets::PepperManager,
}

impl Sec {
    pub fn from_config(config: &config::Config) -> error::Result<Sec> {
        let mut password_key = chacha::empty_key();

        config.kdf.expand(
            rfs_lib::sec::secrets::PASSWORDS_KEY_INFO,
            &mut password_key
        ).map_err(|_| error::Error::new()
            .kind("KDFExpandFailed")
            .message("failed to expand passwords key for secrets manager"))?;

        let peppers = secrets::PepperManager::load(
            config.settings.data.join("sec/secrets/passwords.data"),
            password_key
        ).map_err(|e| error::Error::new()
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

    pub fn peppers(&self) -> &secrets::PepperManager {
        &self.peppers
    }
}

