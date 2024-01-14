use std::default::Default;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use reqwest::Url;
use reqwest::blocking::RequestBuilder;
use reqwest_cookie_store::{CookieStore, CookieStoreRwLock};

pub mod error;
pub mod auth;
pub mod users;
pub mod sec;

use error::ApiClientError;

pub struct Info {
    pub url: Url
}

pub struct ApiClient {
    pub(crate) cookie_file: Option<Box<Path>>,
    pub(crate) store: Arc<CookieStoreRwLock>,
    pub(crate) client: reqwest::blocking::Client,
    pub(crate) info: Info
}

impl ApiClient {
    pub fn builder() -> ApiClientBuilder {
        ApiClientBuilder {
            url: Url::parse("http://localhost/").unwrap(),
            file: None,
            exists: false,
            agent: None
        }
    }

    pub(crate) fn get<U>(&self, path: U) -> RequestBuilder
    where
        U: AsRef<str>,
    {
        let url = self.info.url.join(path.as_ref()).unwrap();

        self.client.get(url)
    }

    pub(crate) fn post<U>(&self, path: U) -> RequestBuilder
    where
        U: AsRef<str>
    {
        let url = self.info.url.join(path.as_ref()).unwrap();

        self.client.post(url)
    }

    pub(crate) fn put<U>(&self, path: U) -> RequestBuilder
    where
        U: AsRef<str>
    {
        let url = self.info.url.join(path.as_ref()).unwrap();

        self.client.put(url)
    }

    pub(crate) fn patch<U>(&self, path: U) -> RequestBuilder
    where
        U: AsRef<str>
    {
        let url = self.info.url.join(path.as_ref()).unwrap();

        self.client.patch(url)
    }

    pub(crate) fn delete<U>(&self, path: U) -> RequestBuilder
    where
        U: AsRef<str>
    {
        let url = self.info.url.join(path.as_ref()).unwrap();

        self.client.delete(url)
    }

    pub fn save_session(&self) -> Result<bool, ApiClientError> {
        let Some(cookie_file) = &self.cookie_file else {
            return Ok(false);
        };

        let store = self.store.read()
            .map_err(|_e| ApiClientError::PoisonedLock)?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(cookie_file)
            .map_err(|e| ApiClientError::StdIo(e))?;
        let mut writer = std::io::BufWriter::new(file);

        store.save_json(&mut writer)
            .map_err(|e| ApiClientError::CookieStore(e))?;

        Ok(true)
    }
}

pub struct ApiClientBuilder {
    url: Url,
    file: Option<PathBuf>,
    exists: bool,
    agent: Option<String>
}

impl ApiClientBuilder {
    pub fn secure(&mut self, is_secure: bool) {
        if is_secure {
            self.url.set_scheme("https").unwrap();
        } else {
            self.url.set_scheme("http").unwrap();
        }
    }

    pub fn host<H>(&mut self, host: H) -> bool
    where
        H: AsRef<str>
    {
        self.url.set_host(Some(host.as_ref())).is_ok()
    }

    pub fn port(&mut self, port: Option<u16>) {
        self.url.set_port(port).unwrap()
    }

    pub fn cookie_file(&mut self, path: PathBuf) {
        self.file = Some(path);
    }

    pub fn cookie_file_exits(&mut self, exists: bool) {
        self.exists = exists;
    }

    pub fn user_agent<U>(&mut self, user_agent: U)
    where
        U: Into<String>
    {
        self.agent = Some(user_agent.into());
    }

    pub fn build(self) -> Result<ApiClient, ApiClientError> {
        let user_agent = self.agent.unwrap_or("rfs-api-client/0.1.0".into());
        let store = if let Some(path) = &self.file {
            match std::fs::OpenOptions::new()
                .read(true)
                .open(&path) {
                Ok(file) => {
                    let reader = std::io::BufReader::new(file);

                    CookieStore::load_json(reader)
                        .map_err(|e| ApiClientError::CookieStore(e))?
                },
                Err(err) => match err.kind() {
                    std::io::ErrorKind::NotFound => {
                        if self.exists {
                            return Err(ApiClientError::StdIo(err));
                        } else {
                            CookieStore::default()
                        }
                    },
                    _ => {
                        return Err(ApiClientError::StdIo(err));
                    }
                }
            }
        } else {
            CookieStore::default()
        };

        let store = Arc::new(CookieStoreRwLock::new(store));
        let client = reqwest::blocking::Client::builder()
            .cookie_provider(store.clone())
            .user_agent(user_agent)
            .build()
            .map_err(|e| ApiClientError::Reqwest(e))?;

        Ok(ApiClient {
            cookie_file: self.file.map(|v| v.into_boxed_path()),
            store,
            client,
            info: Info {
                url: self.url
            }
        })
    }
}
