use rfs_lib::serde::from_to_str;

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordListItem {
    #[serde(with = "from_to_str")]
    pub version: u64,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListItem {
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
    pub created: DateTime<Utc>,
    pub data: Vec<u8>,
}


