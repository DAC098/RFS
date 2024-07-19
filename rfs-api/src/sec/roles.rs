use std::cmp::Ordering;

use rfs_lib::ids;
use rfs_lib::serde::from_to_str;
use rfs_lib::sec::authz::permission::{Ability, Scope};

use snowcloud_flake::serde_ext::string_id;
use serde::{Serialize, Deserialize};

use crate::{Validator, ApiError, ApiErrorKind, Detail};

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleListItem {
    #[serde(with = "from_to_str")]
    pub id: ids::RoleId,
    pub name: String
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Permission {
    pub scope: Scope,
    pub ability: Ability,
}

impl From<(Scope, Ability)> for Permission {
    fn from((scope, ability): (Scope, Ability)) -> Self {
        Permission { scope, ability }
    }
}

impl Ord for Permission {
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp = self.scope.cmp(&other.scope);

        match cmp {
            Ordering::Equal => self.ability.cmp(&other.ability),
            _ => cmp
        }
    }
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

impl Validator for CreateRole {
    fn validate(&self) -> Result<(), ApiError> {
        if !rfs_lib::sec::authz::permission::role_name_valid(&self.name) {
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
pub struct UpdateRole {
    pub name: Option<String>,
    pub permissions: Option<Vec<Permission>>,
}

impl Validator for UpdateRole {
    fn validate(&self) -> Result<(), ApiError> {
        if let Some(name) = &self.name {
            if !rfs_lib::sec::authz::permission::role_name_valid(name) {
                return Err(ApiError::from((
                    ApiErrorKind::ValidationFailed,
                    Detail::with_key("name")
                )));
            }
        }

        Ok(())
    }

    fn has_work(&self) -> bool {
        self.name.is_some() ||
            self.permissions.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddRoleUser {
    pub ids: Vec<ids::UserId>,
}

impl Validator for AddRoleUser {
    fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropRoleUser {
    pub ids: Vec<ids::UserId>,
}

impl Validator for DropRoleUser {
    fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddRoleGroup {
    pub ids: Vec<ids::GroupId>,
}

impl Validator for AddRoleGroup {
    fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropRoleGroup {
    pub ids: Vec<ids::GroupId>,
}

impl Validator for DropRoleGroup {
    fn has_work(&self) -> bool {
        !self.ids.is_empty()
    }
}
