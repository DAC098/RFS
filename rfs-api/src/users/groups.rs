use rfs_lib::ids;

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{Validator, ApiError, ApiErrorKind, Detail};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItem {
    pub uid: ids::GroupUid,
    pub name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub uid: ids::GroupUid,
    pub name: String,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupUser {
    pub uid: ids::UserUid
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
    pub uids: Vec<ids::UserUid>,
}

impl Validator for AddUsers {
    fn has_work(&self) -> bool {
        !self.uids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropUsers {
    pub uids: Vec<ids::UserUid>,
}

impl Validator for DropUsers {
    fn has_work(&self) -> bool {
        !self.uids.is_empty()
    }
}
