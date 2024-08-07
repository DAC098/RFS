use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::error;
use crate::config;
use crate::template;
use crate::sec;

pub mod db;

#[derive(Debug)]
pub struct Shared {
    assets: Assets,
    pages: PathBuf,
    tmp: PathBuf,
    pool: Pool,
    templates: template::state::Templates,
    sec: sec::state::Sec,
}

pub type ArcShared = Arc<Shared>;

impl Shared {
    pub fn from_config(config: &config::Config) -> error::Result<Shared> {
        tracing::debug!("creating Shared state");

        Ok(Shared {
            assets: Assets {
                files: config.settings.assets.files.clone(),
                directories: config.settings.assets.directories.clone(),
            },
            pages: PathBuf::new(),
            tmp: config.settings.tmp.clone(),
            pool: db::from_config(config)?,
            templates: template::state::Templates::from_config(config)?,
            sec: sec::state::Sec::from_config(config)?,
        })
    }

    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn pages(&self) -> &PathBuf {
        &self.pages
    }

    pub fn tmp(&self) -> &Path {
        &self.tmp
    }

    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub fn templates(&self) -> &template::state::Templates {
        &self.templates
    }

    pub fn sec(&self) -> &sec::state::Sec {
        &self.sec
    }

    #[inline]
    pub fn auth(&self) -> &sec::state::Sec {
        self.sec()
    }
}

impl AsRef<Pool> for Shared {
    fn as_ref(&self) -> &Pool {
        &self.pool
    }
}

impl AsRef<sec::state::Sec> for Shared {
    fn as_ref(&self) -> &sec::state::Sec {
        &self.sec
    }
}

impl AsRef<template::state::Templates> for Shared {
    fn as_ref(&self) -> &template::state::Templates {
        &self.templates
    }
}

#[derive(Debug)]
pub struct Assets {
    pub files: HashMap<String, PathBuf>,
    pub directories: HashMap<String, PathBuf>,
}
