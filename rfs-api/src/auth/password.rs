use serde::{Serialize, Deserialize};

use crate::{ApiError, ApiErrorKind, Detail};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePassword {
    pub current: Option<String>,
    pub updated: String,
    pub confirm: String,
}

impl CreatePassword {
    pub fn validate(&self) -> Result<(), ApiError> {
        let mut invalid = Vec::new();

        if let Some(current) = &self.current {
            if !rfs_lib::sec::authn::password_valid(current) {
                invalid.push("current");
            }
        }

        if !rfs_lib::sec::authn::password_valid(&self.updated) {
            invalid.push("updated");
        }

        if !invalid.is_empty() {
            return Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::mult_keys(invalid)
            )));
        }

        if self.updated != self.confirm {
            return Err(ApiError::from((
                ApiErrorKind::InvalidData,
                Detail::with_key("confirm")
            )));
        }

        Ok(())
    }
}
