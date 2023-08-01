use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RequestUser {
    pub username: String
}

#[derive(Serialize, Deserialize)]
pub enum SubmitAuth {
    None,
    Password(String)
}

#[derive(Serialize, Deserialize)]
pub enum SubmitVerify {
    None,
    Totp(String),
    TotpHash(String),
}

#[derive(Serialize, Deserialize)]
pub struct CreatePassword {
    pub current: Option<String>,
    pub updated: String,
    pub confirm: String,
}

#[derive(Serialize, Deserialize)]
pub struct DeletePassword {
    pub current: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateTotp {
    pub algo: Option<String>,
    pub digits: Option<u32>,
    pub step: Option<u64>
}

#[derive(Serialize, Deserialize)]
pub struct UpdateTotp {
    pub algo: Option<String>,
    pub digits: Option<u32>,
    pub step: Option<u64>,
    pub regen: bool
}

impl UpdateTotp {
    pub fn has_work(&self) -> bool {
        self.algo.is_some() ||
            self.digits.is_some() ||
            self.step.is_some() ||
            self.regen
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateTotpHash {
    pub key: String
}

#[derive(Serialize, Deserialize)]
pub struct UpdateTotpHash {
    pub key: Option<String>,
    pub regen: bool
}
