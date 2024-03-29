use serde::{Serialize, Deserialize};
use rfs_lib::ids;

#[derive(Debug, Serialize, Deserialize)]
pub struct Local {
    pub id: ids::StorageId
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Storage {
    Local(Local)
}

impl Storage {
    pub fn id(&self) -> &ids::StorageId {
        match self {
            Storage::Local(local) => &local.id
        }
    }

    pub fn into_schema(self) -> rfs_api::fs::Storage {
        match self {
            Storage::Local(local) => rfs_api::fs::Storage::Local {
                id: local.id
            }
        }
    }
}

impl From<Local> for Storage {
    fn from(v: Local) -> Storage {
        Storage::Local(v)
    }
}
