use std::default::Default;
use std::path::PathBuf;
use std::sync::Arc;

use reqwest::Url;

use crate::error;

pub struct ServerInfo {
    pub url: Url,
}

impl ServerInfo {
    pub fn new() -> Self {
        ServerInfo {
            url: Url::parse("http://localhost:80").unwrap()
        }
    }
}

type CookieStore = reqwest_cookie_store::CookieStore;
type CookieStoreSync = reqwest_cookie_store::CookieStoreRwLock;

pub struct AppState {
    pub cookie_file: PathBuf,
    pub store: Arc<CookieStoreSync>,
    pub client: reqwest::blocking::Client,
    pub server: ServerInfo,
}

impl AppState {
    pub fn load<P>(given_file: P) -> error::Result<Self>
    where
        P: AsRef<std::path::Path>
    {
        let given_file_ref = given_file.as_ref();

        let store = if given_file_ref.try_exists()? {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(given_file_ref)?;
            let reader = std::io::BufReader::new(file);

            CookieStore::load_json(reader)
                .map_err(|e| error::Error::new()
                    .kind("FailedLoadingRFSCookies")
                    .message("failed to load the requested cookies file")
                    .source(e))?
        } else {
            CookieStore::default()
        };

        let store = Arc::new(CookieStoreSync::new(store));
        let client = reqwest::blocking::Client::builder()
            .cookie_provider(store.clone())
            .user_agent("rfs-client-0.1.0")
            .build()
            .expect("failed to create client");

        Ok(AppState {
            cookie_file: given_file_ref.to_owned(),
            store,
            client,
            server: ServerInfo::new(),
        })
    }

    pub fn save(&self) -> error::Result<()> {
        let store = self.store.read()
            .map_err(|_e| error::Error::new()
                .kind("RwLockPoisoned")
                .message("something has caused the RwLock to be poisoned"))?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.cookie_file)
            .map_err(|e| error::Error::new()
                .kind("FailedSavingRFSCookies")
                .message("failed to open the rfs cookies file")
                .source(e))?;
        let mut writer = std::io::BufWriter::new(file);

        store.save_json(&mut writer)
            .map_err(|e| error::Error::new()
                .kind("FailedSavingRFSCookies")
                .message("failed to save data to the rfs cookies file")
                .source(e))?;

        Ok(())
    }
}
