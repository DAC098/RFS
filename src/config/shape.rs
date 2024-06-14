use std::net::{SocketAddr, IpAddr};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::str::FromStr;

use serde::Deserialize;

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

pub mod sec {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub enum Hash {
        Blake3,
        HS256,
        HS384,
        HS512,
    }

    #[derive(Debug, Deserialize)]
    pub struct Session {
        pub hash: Option<Hash>,
        pub secure: Option<bool>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    pub enum Secrets {
        Local {}
    }

    #[derive(Debug, Deserialize)]
    pub struct Sec {
        pub session: Option<Session>,
        pub secrets: Option<Secrets>,
    }
}

#[derive(Debug, Deserialize)]
pub struct ListenerTls {
    pub key: PathBuf,
    pub cert: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Listener {
    pub addr: String,
    pub tls: Option<ListenerTls>,
}

pub use sec::Sec;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub id: Option<i64>,
    pub data: Option<PathBuf>,
    pub master_key: Option<String>,

    pub listeners: Option<HashMap<String, Listener>>,

    pub templates: Option<Templates>,
    pub assets: Option<Assets>,

    pub sec: Option<Sec>,
    pub db: Option<Db>,
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
            _ => Err(error::Error::from(err)
                .message(format!("io error when attempting to resolve a file path ({})", name)))
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
            _ => Err(error::Error::from(err)
                .message(format!("io error when attempting to retrieve path metadata ({})", name)))
        }
    }
}

pub fn from_file(settings_path: &PathBuf) -> error::Result<Settings> {
    let Some(ext) = settings_path.extension() else {
        return Err(error::Error::new()
            .kind("UnknownConfigType")
            .message("failed to retrieve the file extension of the config file"));
    };

    let ext = ext.to_ascii_lowercase();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(settings_path)
        .map_err(|e| error::Error::from(e)
            .kind("FailedOpeningConfig")
            .message("failed to open the specified config file"))?;
    let reader = std::io::BufReader::new(file);

    if ext.eq("yaml") || ext.eq("yml") {
        serde_yaml::from_reader(reader)
            .map_err(|e| error::Error::from(e)
                .kind("FailedParsingYaml")
                .message("there was an error when attempting to parse the yaml config file"))
    } else if ext.eq("json") {
        serde_json::from_reader(reader)
            .map_err(|e| error::Error::from(e)
                .kind("FailedParsingJson")
                .message("there was an error when attempting to parse the json config file"))
    } else {
        Err(error::Error::new()
            .kind("InvalidConfigType")
            .message("the specified config type is not yaml or json"))
    }
}

pub fn validate(settings_path: &PathBuf, mut settings: Settings) -> error::Result<Settings> {
    settings.listeners = if let Some(listeners) = settings.listeners {
        let mut verified = HashMap::with_capacity(listeners.len());

        for (key, mut value) in listeners {
            let name = format!("config.listeners.\"{key}\".addr");

            if let Err(_err) = SocketAddr::from_str(&value.addr) {
                if let Err(_err) = IpAddr::from_str(&value.addr) {
                    return Err(error::Error::new()
                        .kind("InvalidConfig")
                        .message(format!("{name} is not a valid addr/ip address")));
                }
            }

            value.tls = if let Some(mut tls) = value.tls {
                let key_name = format!("config.listeners.\"{key}\".tls.key");
                let cert_name = format!("config.listeners.\"{key}\".tls.cert");

                tls.key = resolve_path(tls.key, &settings_path, &key_name)?;
                tls.cert = resolve_path(tls.cert, &settings_path, &cert_name)?;

                Some(tls)
            } else {
                None
            };

            verified.insert(key, value);
        }

        Some(verified)
    } else {
        None
    };

    settings.data = if let Some(p) = settings.data {
        Some(resolve_path(p, &settings_path, "config.data")?)
    } else {
        None
    };

    settings.assets = if let Some(mut assets) = settings.assets {
        assets.files = if let Some(files) = assets.files {
            let mut verified = HashMap::with_capacity(files.len());

            for (key, value) in files {
                let name = format!("config.assets.files.\"{}\"", key.display());
                let resolved = resolve_path(value, &settings_path, &name)?;
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
                let resolved = resolve_path(value, &settings_path, &name)?;
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

    settings.templates = if let Some(mut templates) = settings.templates {
        templates.directory = if let Some(directory) = templates.directory {
            let resolved = resolve_path(directory, &settings_path, "config.templates.directory")?;
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

    Ok(settings)
}
