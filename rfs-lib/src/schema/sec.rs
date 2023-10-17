use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::serde::from_to_str;

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordListItem {
    #[serde(with = "from_to_str")]
    pub version: u64,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListItem {
    #[serde(with = "from_to_str")]
    pub version: u64,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordVersion {
    #[serde(with = "from_to_str")]
    pub version: u64,
    pub created: DateTime<Utc>,
    pub data: Vec<u8>,
    #[serde(with = "from_to_str")]
    pub in_use: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionVersion {
    #[serde(with = "from_to_str")]
    pub version: u64,
    pub created: DateTime<Utc>,
    pub data: Vec<u8>,
    #[serde(with = "from_to_str")]
    pub in_use: u64,
}
