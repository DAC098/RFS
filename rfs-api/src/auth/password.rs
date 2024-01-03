use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePassword {
    pub current: Option<String>,
    pub updated: String,
    pub confirm: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletePassword {
    pub current: Option<String>
}
