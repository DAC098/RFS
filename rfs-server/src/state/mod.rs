use std::path::PathBuf;
use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::error;
use crate::config;
use crate::template;
use crate::sec;
use crate::fs;

pub mod ids;
pub mod db;

#[derive(Debug)]
pub struct Shared {
    assets: PathBuf,
    pages: PathBuf,
    pool: Pool,
    templates: template::state::Templates,
    sec: sec::state::Sec,
    ids: ids::Ids,
}

pub type ArcShared = Arc<Shared>;

impl Shared {
    pub fn from_config(config: &config::Config) -> error::Result<Shared> {
        tracing::debug!("creating Shared state");

        Ok(Shared {
            assets: PathBuf::new(),
            pages: PathBuf::new(),
            pool: db::from_config(config)?,
            templates: template::state::Templates::from_config(config)?,
            sec: sec::state::Sec::from_config(config)?,
            ids: ids::Ids::new(config.settings.id)?
        })
    }

    pub fn assets(&self) -> &PathBuf {
        &self.assets
    }

    pub fn pages(&self) -> &PathBuf {
        &self.pages
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

    pub fn ids(&self) -> &ids::Ids {
        &self.ids
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

impl AsRef<ids::Ids> for Shared {
    fn as_ref(&self) -> &ids::Ids {
        &self.ids
    }
}

impl AsRef<template::state::Templates> for Shared {
    fn as_ref(&self) -> &template::state::Templates {
        &self.templates
    }
}
