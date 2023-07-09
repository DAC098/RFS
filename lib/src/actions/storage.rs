use std::path::PathBuf;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateStorageType {
    Local {
        path: PathBuf
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStorage {
    pub name: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: CreateStorageType,
    pub tags: HashMap<String, Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UpdateStorageType {
    Local {}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStorage {
    pub name: Option<String>,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub type_: Option<UpdateStorageType>,
    pub tags: Option<HashMap<String, Option<String>>>,
}

impl UpdateStorage {
    pub fn has_work(&self) -> bool {
        self.name.is_some() ||
            self.type_.is_some() ||
            self.tags.is_some()
    }
}
