use super::ids::UserId;

pub struct UserEmail {
    pub address: String,
    pub verified: bool,
}

pub enum UserAuth {
    Password {
        hash: String
    }
}

pub enum UserMFA {
    Totp {
        algo: String,
        step: u64,
        digits: u8,
        secret: String,
    },
    TotpHash {
        key: String,
        used: bool,
    },
}

pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: Option<UserEmail>,
    pub auth: Option<UserAuth>,
    pub mfa: Option<UserMFA>,
    pub created: String,
    pub updated: Option<String>,
    pub deleted: Option<String>,
}

