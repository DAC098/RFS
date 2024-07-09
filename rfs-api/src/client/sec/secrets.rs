use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{Payload, ApiError, ApiErrorKind};
use crate::sec::secrets::{
    PasswordListItem,
    PasswordVersion,
    SessionListItem,
};

pub struct QueryPasswordSecrets {}

impl QueryPasswordSecrets {
    pub fn new() -> Self {
        QueryPasswordSecrets {}
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Vec<PasswordListItem>>, RequestError> {
        let res = client.get("/api/sec/secrets/password").send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct RetrievePasswordSecret {
    version: u64
}

impl RetrievePasswordSecret {
    pub fn version(version: u64) -> Self {
        RetrievePasswordSecret { version }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<PasswordVersion>>, RequestError> {
        let res = client.get(format!("/api/sec/secrets/password/{}", self.version)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::SecretNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            },
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct QuerySessionSecrets {}

impl QuerySessionSecrets {
    pub fn new() -> Self {
        QuerySessionSecrets {}
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Vec<SessionListItem>>, RequestError> {
        let res = client.get("/api/sec/secrets/session").send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreatePasswordSecret {}

impl CreatePasswordSecret {
    pub fn new() -> Self {
        CreatePasswordSecret {}
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.post("/api/sec/secrets/password").send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateSessionSecret {}

impl CreateSessionSecret {
    pub fn new() -> Self {
        CreateSessionSecret {}
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.post("/api/sec/secrets/session").send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeletePasswordSecret {
    version: u64
}

impl DeletePasswordSecret {
    pub fn version(version: u64) -> Self {
        DeletePasswordSecret { version }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/api/sec/secrets/password/{}", self.version))
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteSessionSecret {
    amount: usize
}

impl DeleteSessionSecret {
    pub fn amount(amount: usize) -> Self {
        DeleteSessionSecret { amount }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete("/api/sec/secrets/session")
            .query(&[("amount", self.amount)])
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
