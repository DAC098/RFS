use serde::{Serialize, Deserialize};

use crate::ids;
use crate::sec::authz::permission::{Ability, Scope};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub permissions: Vec<RolePermission>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRole {
    pub name: String,
    pub permissions: Vec<RolePermission>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RolePermission {
    pub scope: Scope,
    pub ability: Ability,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddRoleUser {
    pub ids: Vec<ids::UserId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropRoleUser {
    pub ids: Vec<ids::UserId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddRoleGroup {
    pub ids: Vec<ids::GroupId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropRoleGroup {
    pub ids: Vec<ids::GroupId>,
}
