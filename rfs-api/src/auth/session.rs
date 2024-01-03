use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestUser {
    pub username: String
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestedAuth {
    Password
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmittedAuth {
    Password(String)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestedVerify {
    Topt {
        digits: u32
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmittedVerify {
    Totp(String),
    TotpHash(String)
}
