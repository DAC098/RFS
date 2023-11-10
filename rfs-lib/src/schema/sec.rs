use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use snowcloud_flake::serde_ext::string_id;

use crate::ids;
use crate::sec;
use crate::serde::from_to_str;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleListItem {
    #[serde(with = "from_to_str")]
    pub id: ids::RoleId,
    pub name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Permission {
    pub scope: sec::authz::permission::Scope,
    pub ability: sec::authz::permission::Ability,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    #[serde(with = "from_to_str")]
    pub id: i64,
    pub name: String,
    pub permissions: Vec<Permission>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleUser {
    #[serde(with = "string_id")]
    pub id: ids::UserId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleGroup {
    #[serde(with = "from_to_str")]
    pub id: ids::GroupId
}
