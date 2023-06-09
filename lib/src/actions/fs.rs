use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct UpdateFS {
    pub tags: Vec<String>
}
