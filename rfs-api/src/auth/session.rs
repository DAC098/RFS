use serde::{Serialize, Deserialize};

use crate::{ApiError, ApiErrorKind, Detail};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestUser {
    pub username: String
}

impl RequestUser {
    pub fn validate(&self) -> Result<(), ApiError> {
        if !rfs_lib::user::username_valid(&self.username) {
            return Err(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("username")
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestedAuth {
    Password
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmittedAuth {
    Password(String)
}

impl SubmittedAuth {
    pub fn validate(&self) -> Result<(), ApiError> {
        match self {
            SubmittedAuth::Password(v) => {
                if !rfs_lib::sec::authn::password_valid(&v) {
                    return Err(ApiError::from((
                        ApiErrorKind::ValidationFailed,
                        Detail::with_key("password")
                    )));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestedVerify {
    Totp {
        digits: u32
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmittedVerify {
    Totp(String),
    TotpHash(String)
}

impl SubmittedVerify {
    pub fn validate(&self) -> Result<(), ApiError> {
        match self {
            SubmittedVerify::Totp(v) => {
                for ch in v.chars() {
                    if !ch.is_ascii_digit() {
                        return Err(ApiError::from((
                            ApiErrorKind::ValidationFailed,
                            Detail::with_key("totp")
                        )));
                    }
                }
            },
            SubmittedVerify::TotpHash(_v) => {}
        }

        Ok(())
    }
}
