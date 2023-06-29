use serde::{Serialize, Deserialize};
use lib::ids;
use lib::schema;

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

    pub fn into_schema(self) -> schema::fs::Storage {
        match self {
            Storage::Local(local) => schema::fs::Storage::Local {
                id: local.id
            }
        }
    }
}
