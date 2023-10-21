use std::path::PathBuf;
use std::sync::RwLock;

use rfs_lib::sec::chacha;
use rust_lib_history::versioned::Versioned;
use rust_lib_history::list::fixed::Fixed;
use serde::Deserialize;

use crate::error;
use crate::config;

use super::secrets;

#[derive(Debug)]
pub struct SessionInfo {
    manager: secrets::SessionWrapper,
    domain: Option<String>,
    secure: bool,
}

impl SessionInfo {
    pub fn from_config(config: &config::Config) -> error::Result<Self> {
        tracing::debug!("creating SessionInfo state");

        let mut session_key = chacha::empty_key();

        config.kdf.expand(
            rfs_lib::sec::secrets::SESSIONS_KEY_INFO,
            &mut session_key
        ).map_err(|_| error::Error::new()
            .kind("KDFExpandFailed")
            .message("failed to expand session key for secrets manager"))?;

        let secrets_file = config.settings.data.join("sec/secrets/session.data");

        let manager = secrets::SessionWrapper::load_create(secrets_file, session_key)
            .map_err(|e| error::Error::new()
                .kind("SessionWrapperFailed")
                .message("failed to save new session secrets file")
                .source(e))?;

        Ok(SessionInfo {
            manager,
            domain: None,
            secure: config.settings.sec.session.secure
        })
    }

    pub fn keys(&self) -> &secrets::SessionWrapper {
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
    peppers: secrets::PepperWrapper,
}

impl Sec {
    pub fn from_config(config: &config::Config) -> error::Result<Sec> {
        tracing::debug!("creating Sec state");

        let mut password_key = chacha::empty_key();

        config.kdf.expand(
            rfs_lib::sec::secrets::PASSWORDS_KEY_INFO,
            &mut password_key
        ).map_err(|_| error::Error::new()
            .kind("KDFExpandFailed")
            .message("failed to expand passwords key for secrets manager"))?;

        let secrets_file = config.settings.data.join("sec/secrets/passwords.data");

        let peppers = secrets::PepperWrapper::load_create(secrets_file, password_key)
            .map_err(|e| error::Error::new()
                .kind("PepperWrapperFailed")
                .message("failed to save new pepper secrets file")
                .source(e))?;

        Ok(Sec {
            session_info: SessionInfo::from_config(config)?,
            peppers
        })
    }

    pub fn session_info(&self) -> &SessionInfo {
        &self.session_info
    }

    pub fn peppers(&self) -> &secrets::PepperWrapper {
        &self.peppers
    }
}

