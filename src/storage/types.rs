use std::path::PathBuf;

use rfs_lib::schema::storage::{StorageType, StorageLocal};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Local {
    pub path: PathBuf,
}

impl Local {
    pub fn into_schema(self) -> StorageLocal {
        StorageLocal {
            path: self.path
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Type {
    Local(Local)
}

impl Type {
    pub fn into_schema(self) -> StorageType {
        match self {
            Type::Local(local) => StorageType::Local(local.into_schema())
        }
    }
}

impl From<Local> for Type {
    fn from(local: Local) -> Self {
        Type::Local(local)
    }
}
