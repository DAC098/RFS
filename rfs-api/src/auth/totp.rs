use rfs_lib::ids;

use serde::{Serialize, Deserialize};

use crate::{ApiError, ApiErrorKind, Detail};

pub use rfs_lib::sec::authn::totp::Algo;

#[derive(Debug, Serialize, Deserialize)]
pub struct Totp {
    pub algo: String,
    pub secret: Vec<u8>,
    pub digits: u32,
    pub step: u64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TotpRecovery {
    pub user_id: ids::UserId,
    pub key: String,
    pub hash: String,
    pub used: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTotp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algo: Option<Algo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u64>
}

impl CreateTotp {
    pub fn validate(&self) -> Result<(), ApiError> {
        let mut invalid = Vec::new();

        if let Some(digits) = &self.digits {
            if !rfs_lib::sec::authn::totp::digits_valid(digits) {
                invalid.push("digits");
            }
        }

        if let Some(step) = &self.step {
            if !rfs_lib::sec::authn::totp::step_valid(step) {
                invalid.push("step");
            }
        }

        if !invalid.is_empty() {
            Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::mult_keys(invalid)
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTotp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algo: Option<Algo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u64>,
    pub regen: bool
}

impl UpdateTotp {
    pub fn validate(&self) -> Result<(), ApiError> {
        let mut invalid = Vec::new();

        if let Some(digits) = &self.digits {
            if !rfs_lib::sec::authn::totp::digits_valid(digits) {
                invalid.push("digits");
            }
        }

        if let Some(step) = &self.step {
            if !rfs_lib::sec::authn::totp::step_valid(step) {
                invalid.push("step");
            }
        }

        if !invalid.is_empty() {
            Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::mult_keys(invalid)
            )))
        } else {
            Ok(())
        }
    }

    pub fn has_work(&self) -> bool {
        self.algo.is_some() ||
            self.digits.is_some() ||
            self.step.is_some() ||
            self.regen
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTotpHash {
    pub key: String
}

impl CreateTotpHash {
    pub fn validate(&self) -> Result<(), ApiError> {
        if !rfs_lib::sec::authn::totp::recovery::key_valid(&self.key) {
            Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("key")
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTotpHash {
    pub key: Option<String>,
    pub regen: bool
}

impl UpdateTotpHash {
    pub fn validate(&self) -> Result<(), ApiError> {
        if let Some(key) = &self.key {
            if !rfs_lib::sec::authn::totp::recovery::key_valid(key) {
                return Err(ApiError::from((
                    ApiErrorKind::ValidationFailed,
                    Detail::with_key("key")
                )));
            }
        }

        Ok(())
    }

    pub fn has_work(&self) -> bool {
        self.key.is_some() ||
            self.regen
    }
}
