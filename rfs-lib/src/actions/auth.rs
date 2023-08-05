use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestUser {
    pub username: String
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmitAuth {
    None,
    Password(String)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmitVerify {
    None,
    Totp(String),
    TotpHash(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePassword {
    pub current: Option<String>,
    pub updated: String,
    pub confirm: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletePassword {
    pub current: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTotp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u64>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTotp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTotpHash {
    pub key: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTotpHash {
    pub key: Option<String>,
    pub regen: bool
}

impl UpdateTotpHash {
    pub fn has_work(&self) -> bool {
        self.key.is_some() ||
            self.regen
    }
}
