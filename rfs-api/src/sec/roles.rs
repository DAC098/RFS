use rfs_lib::ids;
use rfs_lib::serde::from_to_str;
use rfs_lib::sec::authz::permission::{Ability, Scope};

use snowcloud_flake::serde_ext::string_id;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleListItem {
    #[serde(with = "from_to_str")]
    pub id: ids::RoleId,
    pub name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Permission {
    pub scope: Scope,
    pub ability: Ability,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    #[serde(with = "from_to_str")]
    pub id: ids::RoleId,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRole {
    pub name: String,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub permissions: Option<Vec<Permission>>,
}

impl UpdateRole {
    pub fn has_work(&self) -> bool {
        self.name.is_some() ||
            self.permissions.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddRoleUser {
    pub ids: Vec<ids::UserId>,
}

impl AddRoleUser {
    pub fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropRoleUser {
    pub ids: Vec<ids::UserId>,
}

impl DropRoleUser {
    pub fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddRoleGroup {
    pub ids: Vec<ids::GroupId>,
}

impl AddRoleGroup {
    pub fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropRoleGroup {
    pub ids: Vec<ids::GroupId>,
}

impl DropRoleGroup {
    pub fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}
