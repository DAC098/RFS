use std::path::PathBuf;
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Templates {
    pub dev_mode: Option<bool>,
    pub directory: Option<PathBuf>
}

#[derive(Debug, Deserialize)]
pub struct Assets {
    pub files: Option<HashMap<String, PathBuf>>,
    pub directories: Option<HashMap<String, PathBuf>>,
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

#[derive(Debug, Deserialize)]
pub struct Tls {
    pub key: PathBuf,
    pub cert: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Listener {
    pub addr: String,
    pub tls: Option<Tls>,
}

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
