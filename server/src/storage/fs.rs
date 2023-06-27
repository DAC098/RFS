use serde::{Serialize, Deserialize};
use lib::ids;
use lib::models;

#[derive(Debug, Serialize, Deserialize)]
pub struct Local {
    pub id: ids::StorageId
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Storage {
    Local(Local)
}

impl Storage {
    pub fn id(&self) -> &ids::StorageId {
        match self {
            Storage::Local(local) => &local.id
        }
    }

    pub fn into_model(self) -> models::fs::Storage {
        match self {
            Storage::Local(local) => models::fs::Storage::Local {
                id: local.id
            }
        }
    }
}
