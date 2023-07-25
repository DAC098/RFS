use std::path::PathBuf;
use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::error;
use crate::template;
use crate::fs;
use crate::sec;

pub mod ids;
pub mod db;

/// builder for creating the [`Shared`] struct
#[derive(Debug)]
pub struct SharedBuilder {
    primary_id: Option<i64>,
    assets: Option<PathBuf>,
    pg_options: db::Builder,
    templates: template::state::Builder,
    sec: sec::state::Builder,
}

impl SharedBuilder {
    pub fn templates(&mut self) -> &mut template::state::Builder {
        &mut self.templates
    }

    pub fn sec(&mut self) -> &mut sec::state::Builder {
        &mut self.sec
    }

    pub fn pg_options(&mut self) -> &mut db::Builder {
        &mut self.pg_options
    }

    pub fn set_primary_id(&mut self, primary: i64) -> &mut Self {
        self.primary_id = Some(primary);
        self
    }

    pub fn build(self) -> error::Result<Shared> {
        let cwd = std::env::current_dir()?;

        let assets = fs::validate_dir(
            "assets",
            &cwd,
            self.assets.unwrap_or("assets".into())
        )?;

        let pages = fs::validate_dir(
            "pages",
            &cwd,
            "pages"
        )?;

        let primary_id = self.primary_id.unwrap_or(1);

        Ok(Shared {
            assets,
            pages,
            pool: self.pg_options.build()?,
            templates: self.templates.build()?,
            sec: self.sec.build()?,
            ids: ids::Ids::new(primary_id)?,
        })
    }
}

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
    pub fn builder() -> SharedBuilder {
        SharedBuilder {
            primary_id: None,
            assets: None,
            pg_options: db::Builder::new(),
            templates: template::state::Templates::builder(),
            sec: sec::state::Sec::builder(),
        }
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
