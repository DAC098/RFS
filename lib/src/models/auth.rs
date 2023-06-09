use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum VerifyMethod {
    None,
    Totp {
        digits: u32
    }
}

#[derive(Serialize, Deserialize)]
pub enum AuthMethod {
    None,
    Password
}
