use serde::{Serialize, Deserialize};
use snowcloud_flake::serde_ext::string_id;

use crate::ids;

pub mod group;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItem {
    #[serde(with = "string_id")]
    pub id: ids::UserId,
    pub username: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Email {
    pub email: String,
    pub verified: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(with = "string_id")]
    pub id: ids::UserId,
    pub username: String,
    pub email: Option<Email>
}
