use std::path::PathBuf;
use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::error;
use crate::template;
use crate::fs;
use crate::auth;

pub mod ids;
pub mod db;

/// builder for creating the [`Shared`] struct
#[derive(Debug)]
pub struct SharedBuilder {
    primary_id: Option<i64>,
    assets: Option<PathBuf>,
    pages: Option<PathBuf>,
    pg_options: db::Builder,
    templates: template::state::Builder,
    auth: auth::state::Builder,
}

impl SharedBuilder {
    pub fn templates(&mut self) -> &mut template::state::Builder {
        &mut self.templates
    }

    pub fn auth(&mut self) -> &mut auth::state::Builder {
        &mut self.auth
    }

    pub fn pg_options(&mut self) -> &mut db::Builder {
        &mut self.pg_options
    }

    /// assigns a new directory for assets lookup
    pub fn set_assets<P>(&mut self, path: P) -> &mut Self 
    where
        P: Into<PathBuf>
    {
        self.assets = Some(path.into());
        self
    }

    /// assigns a new directory for html page lookup
    pub fn set_pages<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>
    {
        self.pages = Some(path.into());
        self
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
            self.pages.unwrap_or("pages".into())
        )?;

        let primary_id = self.primary_id.unwrap_or(1);

        Ok(Shared {
            assets,
            pages,
            pool: self.pg_options.build()?,
            templates: self.templates.build()?,
            auth: self.auth.build()?,
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
    auth: auth::state::Auth,
    ids: ids::Ids,
}

pub type ArcShared = Arc<Shared>;

impl Shared {
    pub fn builder() -> SharedBuilder {
        SharedBuilder {
            primary_id: None,
            assets: None,
            pages: None,
            pg_options: db::Builder::new(),
            templates: template::state::Templates::builder(),
            auth: auth::state::Auth::builder(),
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

    pub fn auth(&self) -> &auth::state::Auth {
        &self.auth
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

impl AsRef<auth::state::Auth> for Shared {
    fn as_ref(&self) -> &auth::state::Auth {
        &self.auth
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
