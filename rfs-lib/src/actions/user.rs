use serde::{Serialize, Deserialize};

use crate::serde::nested_option;

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
