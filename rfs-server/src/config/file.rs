use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::io::ErrorKind;

use serde::Deserialize;

use crate::sec;
use crate::error;

#[derive(Debug, Deserialize)]
pub struct Templates {
    pub dev_mode: Option<bool>,
    pub directory: Option<PathBuf>
}

#[derive(Debug, Deserialize)]
pub struct Assets {
    pub files: Option<HashMap<PathBuf, PathBuf>>,
    pub directories: Option<HashMap<PathBuf, PathBuf>>,
}

#[derive(Debug, Deserialize)]
pub struct Db {
    pub user: Option<String>,
    pub password: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub dbname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SecSession {
    pub hash: Option<sec::state::SessionHash>,
    pub secure: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Sec {
    pub session: Option<SecSession>
}

#[derive(Debug, Deserialize)]
pub struct Root {
    pub id: Option<i64>,
    pub data: Option<PathBuf>,

    pub ip: Option<String>,
    pub port: Option<u16>,

    pub templates: Option<Templates>,
    pub assets: Option<Assets>,

    pub sec: Option<Sec>,
    pub db: Option<Db>,
}

impl Root {
    pub fn default() -> Root {
        Root {
            id: None,
            data: None,
            ip: None,
            port: None,
            templates: None,
            assets: None,
            sec: None,
            db: None,
        }
    }
}

pub fn resolve_path<B, N>(path: PathBuf, base: B, name: &N) -> error::Result<PathBuf>
where
    B: AsRef<Path>,
    N: std::fmt::Display + ?Sized,
{
    let resolve = if path.is_absolute() {
        path
    } else {
        base.as_ref().join(path)
    };

    match resolve.canonicalize() {
        Ok(p) => Ok(p),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Err(error::Error::new()
                .kind("PathNotFound")
                .message(format!("failed to resolve the desired file path ({})", name))),
            _ => Err(error::Error::new()
                .kind("StdIoError")
                .message(format!("io error when attempting to resolve a file path ({})", name))
                .source(err))
        }
    }
}

pub fn path_metadata<P, N>(path: P, name: &N) -> error::Result<std::fs::Metadata>
where
    P: AsRef<Path>,
    N: std::fmt::Display + ?Sized
{
    match path.as_ref().metadata() {
        Ok(m) => Ok(m),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Err(error::Error::new()
                .kind("PathNotFound")
                .message(format!("failed to retrieve path metadata ({})", name))),
            _ => Err(error::Error::new()
                .kind("StdIoError")
                .message(format!("io error when attempting to retrieve path metadata ({})", name))
                .source(err))
        }
    }
}

pub fn load(path: PathBuf) -> error::Result<Root> {
    let cwd = std::env::current_dir()?;
    let config_path = resolve_path(path, &cwd, "config file path")?;

    let mut config: Root = {
        let Some(ext) = config_path.extension() else {
            return Err(error::Error::new()
                .kind("UnknownConfigType")
                .message("failed to retrieve the file extension of the config file"));
        };

        let ext = ext.to_ascii_lowercase();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&config_path)
            .map_err(|e| error::Error::new()
                .kind("FailedOpeningConfig")
                .message("failed to open the specified config file")
                .source(e))?;
        let reader = std::io::BufReader::new(file);

        if ext.eq("yaml") || ext.eq("yml") {
            serde_yaml::from_reader(reader)
                .map_err(|e| error::Error::new()
                    .kind("FailedParsingYaml")
                    .message("there was an error when attempting to parse the yaml config file")
                    .source(e))?
        } else if ext.eq("json") {
            serde_json::from_reader(reader)
                .map_err(|e| error::Error::new()
                    .kind("FailedParsingJson")
                    .message("there was an error when attempting to parse the json config file")
                    .source(e))?
        } else {
            return Err(error::Error::new()
                .kind("InvalidConfigType")
                .message("the specified config type is not yaml or json"));
        }
    };

    config.data = if let Some(p) = config.data {
        Some(resolve_path(p, &cwd, "config.data")?)
    } else {
        None
    };

    config.assets = if let Some(mut assets) = config.assets {
        assets.files = if let Some(files) = assets.files {
            let mut verified = HashMap::with_capacity(files.len());

            for (key, value) in files {
                let name = format!("config.assets.files.\"{}\"", key.display());
                let resolved = resolve_path(value, &config_path, &name)?;
                let metadata = path_metadata(&resolved, &name)?;

                if !metadata.is_file() {
                    return Err(error::Error::new()
                        .kind("InvalidConfig")
                        .message(format!("{} is not a file", name)));
                }

                verified.insert(key, resolved);
            }

            Some(verified)
        } else {
            None
        };

        assets.directories = if let Some(directories) = assets.directories {
            let mut verified = HashMap::with_capacity(directories.len());

            for (key, value) in directories {
                let name = format!("config.assets.directories.\"{}\"", key.display());
                let resolved = resolve_path(value, &config_path, &name)?;
                let metadata = path_metadata(&resolved, &name)?;

                if !metadata.is_dir() {
                    return Err(error::Error::new()
                        .kind("InvalidConfig")
                        .message(format!("{} is not a directory", name)));
                }

                verified.insert(key, resolved);
            }

            Some(verified)
        } else {
            None
        };

        Some(assets)
    } else {
        None
    };

    config.templates = if let Some(mut templates) = config.templates {
        templates.directory = if let Some(directory) = templates.directory {
            let resolved = resolve_path(directory, &config_path, "config.templates.directory")?;
            let metadata = path_metadata(&resolved, "config.templates.directory")?;

            if !metadata.is_dir() {
                return Err(error::Error::new()
                    .kind("InvalidConfig")
                    .message("config.templates.directory is not a directory"));
            }

            Some(resolved)
        } else {
            None
        };

        Some(templates)
    } else {
        None
    };

    Ok(config)
}
