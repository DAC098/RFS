use rfs_lib::ids;

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateGroup {
    pub name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddUsers {
    pub ids: Vec<ids::UserId>,
}

impl AddUsers {
    pub fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropUsers {
    pub ids: Vec<ids::UserId>,
}

impl DropUsers {
    pub fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}
