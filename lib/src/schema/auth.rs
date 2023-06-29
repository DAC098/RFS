use serde::{Serialize, Deserialize};

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
