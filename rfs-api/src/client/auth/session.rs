use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::Payload;
use crate::auth::session::{
    RequestUser,
    RequestedAuth,
    SubmittedAuth,
    RequestedVerify,
    SubmittedVerify,
};

pub struct RequestAuth {
    body: RequestUser
}

impl RequestAuth {
    pub fn new<U>(username: U) -> Self
    where
        U: Into<String>
    {
        RequestAuth {
            body: RequestUser {
                username: username.into()
            }
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<RequestedAuth>>, RequestError> {
        self.body.validate()?;

        let res = client.post("/api/auth/session/request")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NO_CONTENT => Ok(None),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct SubmitAuth {
    body: SubmittedAuth,
}

impl SubmitAuth {
    pub fn password<P>(password: P) -> Self
    where
        P: Into<String>
    {
        SubmitAuth {
            body: SubmittedAuth::Password(password.into())
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<RequestedVerify>>, RequestError> {
        let res = client.post("/api/auth/session/submit")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NO_CONTENT => Ok(None),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct SubmitVerify {
    body: SubmittedVerify
}

impl SubmitVerify {
    pub fn totp<T>(totp: T) -> Self
    where
        T: Into<String>
    {
        SubmitVerify {
            body: SubmittedVerify::Totp(totp.into())
        }
    }

    pub fn totp_hash<T>(totp_hash: T) -> Self
    where
        T: Into<String>
    {
        SubmitVerify {
            body: SubmittedVerify::TotpHash(totp_hash.into())
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.validate()?;

        let res = client.post("/api/auth/session/verify")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DropSession {}

impl DropSession {
    pub fn new() -> Self {
        DropSession {}
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete("/api/auth/session/drop").send()?;

        if res.status() != reqwest::StatusCode::NO_CONTENT {
            Err(RequestError::Api(res.json()?))
        } else {
            Ok(())
        }
    }
}
