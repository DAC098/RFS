use rfs_lib::ids;
use rfs_lib::serde::nested_option;

use serde::{Serialize, Deserialize};
use snowcloud_flake::serde_ext::string_id;

use crate::{Validator, ApiError, ApiErrorKind, Detail};

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

impl Validator for CreateUser {
    fn validate(&self) -> Result<(), ApiError> {
        let mut invalid = Vec::new();

        if !rfs_lib::user::username_valid(&self.username) {
            invalid.push("username");
        }

        if let Some(email) = &self.email {
            if rfs_lib::user::email_valid(email) {
                invalid.push("email");
            }
        }

        if !invalid.is_empty() {
            Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::mult_keys(invalid)
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    #[serde(default, deserialize_with = "nested_option", skip_serializing_if = "Option::is_none")]
    pub email: Option<Option<String>>,
}

impl Validator for UpdateUser {
    fn validate(&self) -> Result<(), ApiError> {
        let mut invalid = Vec::new();

        if let Some(username) = &self.username {
            if !rfs_lib::user::username_valid(username) {
                invalid.push("username");
            }
        }

        if let Some(maybe_email) = &self.email {
            if let Some(email) = maybe_email {
                if !rfs_lib::user::email_valid(email) {
                    invalid.push("email");
                }
            }
        }

        if !invalid.is_empty() {
            Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::mult_keys(invalid)
            )))
        } else {
            Ok(())
        }
    }

    fn has_work(&self) -> bool {
        self.username.is_some() || self.email.is_some()
    }
}
