use rfs_lib::ids;

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{Validator, ApiError, ApiErrorKind, Detail};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItem {
    pub id: ids::GroupId,
    pub name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: ids::GroupId,
    pub name: String,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupUser {
    pub id: ids::UserId
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGroup {
    pub name: String
}

impl Validator for CreateGroup {
    fn validate(&self) -> Result<(), ApiError> {
        if !rfs_lib::users::groups::name_valid(&self.name) {
            Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("name")
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateGroup {
    pub name: String
}

impl Validator for UpdateGroup {
    fn validate(&self) -> Result<(), ApiError> {
        if !rfs_lib::users::groups::name_valid(&self.name) {
            Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("name")
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddUsers {
    pub ids: Vec<ids::UserId>,
}

impl Validator for AddUsers {
    fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropUsers {
    pub ids: Vec<ids::UserId>,
}

impl Validator for DropUsers {
    fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}
