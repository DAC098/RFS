use serde::{Serialize, Deserialize};

use crate::ids;

#[derive(Debug, Serialize, Deserialize)]
pub enum VerifyMethod {
    None,
    Totp {
        digits: u32
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    None,
    Password
}

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
