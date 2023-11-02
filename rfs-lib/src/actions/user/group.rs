use serde::{Serialize, Deserialize};

use crate::ids;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct DropUsers {
    pub ids: Vec<ids::UserId>,
}

