use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::net::{SocketAddr, IpAddr};
use std::default::Default;

use clap::Parser;

use crate::error;
use crate::state;

pub mod shape;

pub type Kdf = hkdf::Hkdf<sha3::Sha3_512>;

pub struct Config {
    pub settings: Settings,
    pub kdf: Kdf,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// a config path or directory to load file from.
    #[arg(long)]
    config: Vec<PathBuf>
}

#[derive(Debug)]
pub struct Assets {
    pub files: HashMap<PathBuf, PathBuf>,
    pub directories: HashMap<PathBuf, PathBuf>,
}

impl Assets {
    fn try_default() -> error::Result<Self> {
        Ok(Assets {
            files: HashMap::new(),
            directories: HashMap::new(),
        })
    }

    fn merge(&mut self, assets: shape::Assets) -> error::Result<()> {
        if let Some(files) = assets.files {
            for (key, value) in files {
                self.files.insert(key, value);
            }
        }

        if let Some(directories) = assets.directories {
            for (key, value) in directories {
                self.directories.insert(key, value);
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Templates {
    pub dev_mode: bool,
    pub directory: PathBuf,
}

impl Templates {
    fn try_default() -> error::Result<Self> {
        let cwd = std::env::current_dir()?;

        Ok(Templates {
            dev_mode: false,
            directory: cwd.join("templates"),
        })
    }

    fn merge(&mut self, templates: shape::Templates) -> error::Result<()> {
        if let Some(dir) = templates.directory {
            self.directory = dir;
        }

        if let Some(dev_mode) = templates.dev_mode {
            self.dev_mode = dev_mode;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Db {
    pub user: String,
    pub password: Option<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
}

impl Db {
    fn try_default() -> error::Result<Self> {
        Ok(Db {
            user: "postgres".into(),
            password: None,
            host: "localhost".into(),
            port: 5432,
            dbname: "rfs".into(),
        })
    }

    fn merge(&mut self, db: shape::Db) -> error::Result<()> {
        if let Some(user) = db.user {
            self.user = user;
        }

        self.password = db.password;

        if let Some(host) = db.host {
            self.host = host;
        }

        if let Some(port) = db.port {
            self.port = port;
        }

        if let Some(dbname) = db.dbname {
            self.dbname = dbname;
        }

        Ok(())
    }
}

pub mod sec {
    use std::default::Default;

    use crate::error;
    use super::shape;

    #[derive(Debug)]
    pub enum Hash {
        Blake3,
        HS256,
        HS384,
        HS512,
    }

    #[derive(Debug)]
    pub struct Session {
        pub hash: Hash,
        pub secure: bool
    }

    impl Session {
        pub(super) fn try_default() -> error::Result<Self> {
            Ok(Session {
                hash: Hash::Blake3,
                secure: false
            })
        }

        pub(super) fn merge(&mut self, session: shape::sec::Session) -> error::Result<()> {
            if let Some(hash) = session.hash {
                self.hash = match hash {
                    shape::sec::Hash::Blake3 => Hash::Blake3,
                    shape::sec::Hash::HS256 => Hash::HS256,
                    shape::sec::Hash::HS384 => Hash::HS384,
                    shape::sec::Hash::HS512 => Hash::HS512,
                };
            }

            if let Some(secure) = session.secure {
                self.secure = secure;
            }

            Ok(())
        }
    }

    #[derive(Debug)]
    pub enum Secrets {
        Local {}
    }

    impl Secrets {
        pub(super) fn try_default() -> error::Result<Self> {
            Ok(Secrets::Local {})
        }

        pub(super) fn merge(&mut self, secrets: shape::sec::Secrets) -> error::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct Sec {
        pub session: Session,
        pub secrets: Secrets
    }

    impl Sec {
        pub(super) fn try_default() -> error::Result<Self> {
            Ok(Sec {
                session: Session::try_default()?,
                secrets: Secrets::try_default()?,
            })
        }

        pub(super) fn merge(&mut self, sec: shape::Sec) -> error::Result<()> {
            if let Some(session) = sec.session {
                self.session.merge(session)?;
            }

            if let Some(secrets) = sec.secrets {
                self.secrets.merge(secrets)?;
            }

            Ok(())
        }
    }
}

pub use sec::Sec;

#[derive(Debug)]
pub struct Settings {
    pub id: i64,
    pub data: PathBuf,
    pub master_key: String,
    pub ip: String,
    pub port: u16,
    pub templates: Templates,
    pub assets: Assets,
    pub sec: Sec,
    pub db: Db,
}

impl Settings {
    fn try_default() -> error::Result<Self> {
        let cwd = std::env::current_dir()?;

        Ok(Settings {
            id: 1,
            data: cwd.join("data"),
            master_key: "rfs_master_key_secret".into(),
            ip: "0.0.0.0".into(),
            port: 8000,
            templates: Templates::try_default()?,
            assets: Assets::try_default()?,
            sec: Sec::try_default()?,
            db: Db::try_default()?,
        })
    }

    fn merge(&mut self, settings: shape::Settings) -> error::Result<()> {
        if let Some(id) = settings.id {
            self.id = id;
        }

        if let Some(data) = settings.data {
            self.data = data;
        }

        if let Some(master_key) = settings.master_key {
            self.master_key = master_key;
        }

        if let Some(ip) = settings.ip {
            self.ip = ip;
        }

        if let Some(port) = settings.port {
            self.port = port;
        }

        if let Some(templates) = settings.templates {
            self.templates.merge(templates)?;
        }

        if let Some(assets) = settings.assets {
            self.assets.merge(assets)?;
        }

        if let Some(sec) = settings.sec {
            self.sec.merge(sec)?;
        }

        if let Some(db) = settings.db {
            self.db.merge(db)?;
        }

        Ok(())
    }

    pub fn listen_socket(&self) -> SocketAddr {
        let ip = IpAddr::from_str(&self.ip).unwrap();

        SocketAddr::new(ip, self.port)
    }
}

pub fn get_config() -> error::Result<Config> {
    let mut settings = Settings::try_default()?;
    let args = CliArgs::parse();

    for config_path in args.config {
        let loaded = shape::from_file(&config_path)?;

        settings.merge(shape::validate(&config_path, loaded)?)?;
    }

    tracing::debug!("{:#?}", settings);

    let kdf = hkdf::Hkdf::<sha3::Sha3_512>::new(None, settings.master_key.as_bytes());

    Ok(Config {
        settings,
        kdf
    })
}
