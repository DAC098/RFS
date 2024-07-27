use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::net::{SocketAddr, IpAddr};
use std::default::Default;
use std::fmt::{Display, Formatter};

use clap::Parser;

use crate::error::{self, Context};
use crate::path::{metadata, normalize};

mod shape;

pub type Kdf = hkdf::Hkdf<sha3::Sha3_512>;

pub trait TryDefault: Sized {
    type Error;

    fn try_default() -> Result<Self, Self::Error>;
}

#[derive(Debug, Parser)]
#[command(author, version ,about, long_about = None)]
pub struct CliArgs {
    /// a config path or directory to load file from
    #[arg(long)]
    config: Vec<PathBuf>
}

#[derive(Debug)]
pub struct Config {
    pub settings: Settings,
    pub kdf: Kdf,
}

impl Config {
    pub fn from_args(args: CliArgs) -> error::Result<Self> {
        let cwd = std::env::current_dir()
            .context("failed to retrieve cwd for Settings")?;
        let mut settings = Settings::try_default()?;

        for config_path in args.config {
            let full = if config_path.is_absolute() {
                config_path
            } else {
                normalize(cwd.join(config_path))
            };

            tracing::debug!("loading config file \"{}\"", full.display());

            let loaded = Self::load_file(&full)?;
            let src = SrcFile::new(&full)?;
            let dot = DotPath::new(&"settings");

            settings.merge(&src, dot, loaded)?;
        }

        {
            let meta = metadata(&settings.data).context(
                "failed to retrieve metadata for settings.data"
            )?.context(
                "settings.data does not exist"
            )?;

            if !meta.is_dir() {
                return Err(error::Error::new().context(
                    "settings.data is not a directory"
                ));
            }
        }

        {
            let meta = metadata(&settings.tmp).context(
                "failed to retrieve metadata for settings.tmp"
            )?.context(
                "settings.tmp does not exist"
            )?;

            if !meta.is_dir() {
                return Err(error::Error::new().context(
                    "settings.tmp is not a directory"
                ));
            }
        }

        {
            let meta = metadata(&settings.templates.directory).context(
                "failed to retrieve metadata for settings.templates.directory"
            )?.context(
                "settings.templates.directory does not exist"
            )?;

            if !meta.is_dir() {
                return Err(error::Error::new().context(
                    "settings.templates.directory is not a directory"
                ));
            }
        }

        tracing::debug!("{settings:#?}");

        let kdf = hkdf::Hkdf::<sha3::Sha3_512>::new(None, settings.master_key.as_bytes());

        Ok(Config {
            settings,
            kdf
        })
    }

    fn load_file(path: &PathBuf) -> error::Result<shape::Settings> {
        let ext = path.extension().context(format!(
            "failed to retrieve the file extension for config file: \"{}\"", path.display()
        ))?;

        let ext = ext.to_ascii_lowercase();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .context(format!("failed to open config file: \"{}\"", path.display()))?;
        let reader = std::io::BufReader::new(file);

        if ext.eq("yaml") || ext.eq("yml") {
            serde_yaml::from_reader(reader).context(format!(
                "failed to parse yaml config file: \"{}\"", path.display()
            ))
        } else if ext.eq("json") {
            serde_json::from_reader(reader).context(format!(
                "failed to parse json config file: \"{}\"", path.display()
            ))
        } else {
            Err(error::Error::new().context(format!(
                "unknown type of config file: \"{}\"", path.display()
            )))
        }
    }
}

struct SrcFile<'a> {
    parent: &'a Path,
    src: &'a Path,
}

impl<'a> SrcFile<'a> {
    fn new(src: &'a Path) -> error::Result<Self> {
        let parent = src.parent().context(format!(
            "failed to retrieve parent path from source file \"{}\"", src.display()
        ))?;

        Ok(SrcFile {
            parent,
            src
        })
    }
}

impl<'a> Display for SrcFile<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.src.display())
    }
}

struct Quote<'a>(&'a dyn Display);

impl<'a> Display for Quote<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

struct DotPath<'a>(Vec<&'a dyn Display>);

impl<'a> DotPath<'a> {
    fn new(name: &'a (dyn Display)) -> Self {
        DotPath(vec![name])
    }

    fn push(&self, name: &'a (dyn Display)) -> Self {
        let mut path = self.0.clone();
        path.push(name);

        DotPath(path)
    }
}

impl<'a> Display for DotPath<'a> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;

        for name in &self.0 {
            if first {
                write!(fmt, "{name}")?;
                first = false;
            } else {
                write!(fmt, ".{name}")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Settings {
    pub id: i64,
    pub data: PathBuf,
    pub tmp: PathBuf,
    pub master_key: String,
    pub listeners: HashMap<String, Listener>,
    pub templates: Templates,
    pub assets: Assets,
    pub sec: Sec,
    pub db: Db,
}

impl Settings {
    fn merge(&mut self, src: &SrcFile<'_>, dot: DotPath<'_>, settings: shape::Settings) -> error::Result<()> {
        if let Some(id) = settings.id {
            self.id = id;
        }

        if let Some(data) = settings.data {
            self.data = check_path(data, src, dot.push(&"data"), false)?;
        }

        if let Some(tmp) = settings.tmp {
            self.tmp = check_path(tmp, src, dot.push(&"tmp"), false)?;
        }

        if let Some(master_key) = settings.master_key {
            self.master_key = master_key;
        }

        if let Some(listeners) = settings.listeners {
            for (key, listener) in listeners {
                if let Some(found) = self.listeners.get_mut(&key) {
                    found.merge(src, dot.push(&Quote(&key)), listener)?;
                } else {
                    let mut default = Listener::default();
                    default.merge(src, dot.push(&Quote(&key)), listener)?;

                    self.listeners.insert(key, default);
                }
            }
        }

        if let Some(templates) = settings.templates {
            self.templates.merge(src, dot.push(&"templates"), templates)?;
        }

        if let Some(assets) = settings.assets {
            self.assets.merge(src, dot.push(&"assets"), assets)?;
        }

        if let Some(sec) = settings.sec {
            self.sec.merge(src, dot.push(&"sec"), sec)?;
        }

        if let Some(db) = settings.db {
            self.db.merge(src, dot.push(&"db"), db)?;
        }

        Ok(())
    }
}

impl TryDefault for Settings {
    type Error = error::Error;

    fn try_default() -> Result<Self, Self::Error> {
        let cwd = std::env::current_dir()
            .context("failed to retrieve cwd for Settings")?;

        Ok(Settings {
            id: 1,
            data: cwd.join("data"),
            tmp: cwd.join("tmp"),
            master_key: "rfs_master_key_secret".into(),
            listeners: HashMap::new(),
            templates: Templates::try_default()?,
            assets: Assets::default(),
            sec: Sec::default(),
            db: Db::default(),
        })
    }
}

#[derive(Debug)]
pub struct Listener {
    pub addr: SocketAddr,
    pub tls: Option<Tls>,
}

impl Listener {
    fn merge(&mut self, src: &SrcFile<'_>, dot_path: DotPath<'_>, listener: shape::Listener) -> error::Result<()> {
        self.addr = match SocketAddr::from_str(&listener.addr) {
            Ok(valid) => valid,
            Err(_) => match IpAddr::from_str(&listener.addr) {
                Ok(valid) => SocketAddr::from((valid, 8080)),
                Err(_) => {
                    return Err(error::Error::new().context(format!(
                        "{dot_path}.addr invalid: \"{}\" file: {src}", listener.addr
                    )));
                }
            }
        };

        if let Some(tls) = listener.tls {
            if let Some(inner) = &mut self.tls {
                inner.merge(src, dot_path.push(&"tls"), tls)?;
            } else {
                let mut default = Tls::default();
                default.merge(src, dot_path.push(&"tls"), tls)?;

                self.tls = Some(default);
            }
        }

        Ok(())
    }
}

impl Default for Listener {
    fn default() -> Self {
        Listener {
            addr: SocketAddr::from((
                IpAddr::from([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]),
                8080
            )),
            tls: None
        }
    }
}

#[derive(Debug)]
pub struct Tls {
    pub key: PathBuf,
    pub cert: PathBuf,
}

impl Tls {
    fn merge(&mut self, src: &SrcFile<'_>, dot_path: DotPath<'_>, tls: shape::Tls) -> error::Result<()> {
        self.key = check_path(tls.key, src, dot_path.push(&"key"), true)?;
        self.cert = check_path(tls.cert, src, dot_path.push(&"cert"), true)?;

        Ok(())
    }
}

impl Default for Tls {
    fn default() -> Self {
        Tls {
            key: PathBuf::new(),
            cert: PathBuf::new(),
        }
    }
}

#[derive(Debug)]
pub struct Templates {
    pub dev_mode: bool,
    pub directory: PathBuf,
}

impl Templates {
    fn merge(&mut self, src: &SrcFile<'_>, dot_path: DotPath<'_>, templates: shape::Templates) -> error::Result<()> {
        if let Some(dev_mode) = templates.dev_mode {
            self.dev_mode = dev_mode;
        }

        if let Some(directory) = templates.directory {
            self.directory = check_path(directory, src, dot_path.push(&"directory"), false)?;
        }

        Ok(())
    }
}

impl TryDefault for Templates {
    type Error = error::Error;

    fn try_default() -> Result<Self, Self::Error> {
        let cwd = std::env::current_dir()
            .context("failed to retrieve cwd for Templates")?;

        Ok(Templates {
            dev_mode: false,
            directory: cwd.join("templates")
        })
    }
}

#[derive(Debug)]
pub struct Assets {
    pub files: HashMap<String, PathBuf>,
    pub directories: HashMap<String, PathBuf>,
}

impl Assets {
    fn merge(&mut self, src: &SrcFile<'_>, dot: DotPath<'_>, assets: shape::Assets) -> error::Result<()> {
        if let Some(files) = assets.files {
            let files_dot = dot.push(&"files");

            for (url_key, path) in files {
                let key = check_url(url_key.clone(), src, files_dot.push(&Quote(&url_key)))?;

                if let Some(found) = self.files.get_mut(&key) {
                    *found = check_path(path, src, files_dot.push(&Quote(&url_key)), true)?;
                } else {
                    self.files.insert(key, check_path(path, src, files_dot.push(&Quote(&url_key)), true)?);
                }
            }
        }

        if let Some(directories) = assets.directories {
            let dirs_dot = dot.push(&"directories");

            for (url_key, path) in directories {
                let key = check_url(url_key.clone(), src, dirs_dot.push(&Quote(&url_key)))?;

                if let Some(found) = self.directories.get_mut(&key) {
                    *found = check_path(path, src, dirs_dot.push(&Quote(&url_key)), false)?;
                } else {
                    self.directories.insert(key, check_path(path, src, dirs_dot.push(&Quote(&url_key)), false)?);
                }
            }
        }

        Ok(())
    }
}
impl Default for Assets {
    fn default() -> Self {
        Assets {
            files: HashMap::new(),
            directories: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Sec {
    pub session: Session,
    pub secrets: Secrets,
}

impl Sec {
    fn merge(&mut self, src: &SrcFile<'_>, dot: DotPath<'_>, sec: shape::Sec) -> error::Result<()> {
        if let Some(session) = sec.session {
            self.session.merge(src, dot.push(&"session"), session)?;
        }

        Ok(())
    }
}

impl Default for Sec {
    fn default() -> Self {
        Sec {
            session: Default::default(),
            secrets: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Session {
    pub hash: Hash,
    pub secure: bool,
}

impl Session {
    fn merge(&mut self, _src: &SrcFile<'_>, _dot: DotPath<'_>, session: shape::Session) -> error::Result<()> {
        if let Some(hash) = session.hash {
            self.hash = match hash {
                shape::Hash::Blake3 => Hash::Blake3,
                shape::Hash::HS256 => Hash::HS256,
                shape::Hash::HS384 => Hash::HS384,
                shape::Hash::HS512 => Hash::HS512,
            };
        }

        if let Some(secure) = session.secure {
            self.secure = secure;
        }

        Ok(())
    }
}

impl Default for Session {
    fn default() -> Self {
        Session {
            hash: Hash::Blake3,
            secure: true,
        }
    }
}

#[derive(Debug)]
pub enum Hash {
    Blake3,
    HS256,
    HS384,
    HS512,
}

#[derive(Debug)]
pub enum Secrets {
    Local {}
}

impl Default for Secrets {
    fn default() -> Self {
        Secrets::Local {}
    }
}

#[derive(Debug)]
pub struct Db {
    pub user: String,
    pub password: Option<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String
}

impl Db {
    fn merge(&mut self, _src: &SrcFile<'_>, _dot: DotPath<'_>, db: shape::Db) -> error::Result<()> {
        if let Some(user) = db.user {
            self.user = user;
        }

        if let Some(password) = db.password {
            self.password = Some(password);
        }

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

impl Default for Db {
    fn default() -> Self {
        Db {
            user: "postgres".into(),
            password: None,
            host: "localhost".into(),
            port: 5432,
            dbname: "rfs".into(),
        }
    }
}

fn check_path(given: PathBuf, src: &SrcFile<'_>, dot: DotPath<'_>, is_file: bool) -> error::Result<PathBuf> {
    let full = if given.is_absolute() {
        given
    } else {
        normalize(src.parent.join(given))
    };

    tracing::debug!("{dot} {src} checking {}", full.display());

    let meta = metadata(&full).context(format!(
        "{dot} failed to retrieve metadata for: {src}"
    ))?.context(format!(
        "{dot} {src} was not found"
    ))?;

    if is_file {
        if !meta.is_file() {
            return Err(error::Error::new().context(format!(
                "{dot} is not a file in: {src}"
            )));
        }
    } else {
        if !meta.is_dir() {
            return Err(error::Error::new().context(format!(
                "{dot} is not a directory in: {src}"
            )));
        }
    }

    Ok(full)
}

fn check_url(given: String, src: &SrcFile<'_>, dot: DotPath<'_>) -> error::Result<String> {
    let trimmed = given.trim();
    let rtn: String;

    let to_parse = if trimmed.starts_with('/') {
        rtn = trimmed.to_owned();

        format!("http://localhost{trimmed}")
    } else {
        rtn = format!("/{trimmed}");

        format!("http://locahost/{trimmed}")
    };

    let url = url::Url::parse(&to_parse).context(format!(
        "{dot} \"{given}\" is not a valid url path. file: {src}"
    ))?;

    for part in url.path_segments().unwrap() {
        if part == ".." || part == "." {
            return Err(error::Error::new().context(format!(
                "{dot} \"{given}\" is not a valid url path. file: {src}"
            )));
        }
    }

    Ok(rtn)
}
