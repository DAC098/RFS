use rfs_lib::ids;
use rfs_lib::serde::nested_option;

use serde::{Serialize, Deserialize};
use snowcloud_flake::serde_ext::string_id;

pub mod groups;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    #[serde(default, deserialize_with = "nested_option", skip_serializing_if = "Option::is_none")]
    pub email: Option<Option<String>>,
}

impl UpdateUser {
    pub fn has_work(&self) -> bool {
        self.username.is_some() || self.email.is_some()
    }
}
