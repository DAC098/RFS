use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::ids;

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
