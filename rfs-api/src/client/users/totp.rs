use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{Payload, ApiError, ApiErrorKind, Detail};
use crate::users::totp::{
    CreateTotp as CreateTotpBody,
    UpdateTotp as UpdateTotpBody,
    CreateTotpHash,
    UpdateTotpHash,
    TotpRecovery,
    Totp,
    Algo,
};

pub struct RetrieveTotp {}

impl RetrieveTotp {
    pub fn new() -> Self {
        RetrieveTotp {}
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Totp>>, RequestError> {
        let res = client.get("/api/user/totp").send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::TotpNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            },
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateTotp {
    body: CreateTotpBody
}

impl CreateTotp {
    pub fn new() -> Self {
        CreateTotp {
            body: CreateTotpBody {
                algo: None,
                digits: None,
                step: None
            }
        }
    }

    pub fn algo(&mut self, algo: Algo) -> &mut Self {
        self.body.algo = Some(algo);
        self
    }

    pub fn digits(&mut self, digits: u32) -> &mut Self {
        self.body.digits = Some(digits);
        self
    }

    pub fn step(&mut self, step: u64) -> &mut Self {
        self.body.step = Some(step);
        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Totp>, RequestError> {
        let res = client.post("/api/user/totp")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateTotp {
    body: UpdateTotpBody
}

impl UpdateTotp {
    pub fn new() -> Self {
        UpdateTotp {
            body: UpdateTotpBody {
                algo: None,
                digits: None,
                step: None,
                regen: false,
            }
        }
    }

    pub fn algo(&mut self, algo: Algo) -> &mut Self {
        self.body.algo = Some(algo);
        self
    }

    pub fn digits(&mut self, digits: u32) -> &mut Self {
        self.body.digits = Some(digits);
        self
    }

    pub fn step(&mut self, step: u64) -> &mut Self {
        self.body.step = Some(step);
        self
    }

    pub fn regen(&mut self, regen: bool) -> &mut Self {
        self.body.regen = regen;
        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Totp>, RequestError> {
        let res = client.patch("/api/user/totp")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct DeleteTotp {}

impl DeleteTotp {
    pub fn new() -> Self {
        DeleteTotp {}
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete("/api/user/totp").send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct RetrieveTotpRecovery {}

impl RetrieveTotpRecovery {
    pub fn new() -> Self {
        RetrieveTotpRecovery {}
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Vec<TotpRecovery>>, RequestError> {
        let res = client.get("/api/user/totp/recovery").send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateTotpRecovery {
    body: CreateTotpHash
}

impl CreateTotpRecovery {
    pub fn key<K>(key: K) -> Self
    where
        K: Into<String>
    {
        CreateTotpRecovery {
            body: CreateTotpHash {
                key: key.into()
            }
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<TotpRecovery>, RequestError> {
        self.body.validate()?;

        let res = client.post("/api/user/totp/recovery")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct RetrieveTotpRecoveryKey {
    key: String,
}

impl RetrieveTotpRecoveryKey {
    pub fn key<K>(key: K) -> Self
    where
        K: Into<String>
    {
        RetrieveTotpRecoveryKey {
            key: key.into()
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<TotpRecovery>>, RequestError> {
        if !rfs_lib::sec::authn::totp::recovery::key_valid(&self.key) {
            return Err(RequestError::Api(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("key")
            ))));
        }

        let res = client.get(format!("/api/user/totp/recovery/{}", self.key)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::TotpRecoveryNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            },
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateTotpRecovery {
    key: String,
    body: UpdateTotpHash
}

impl UpdateTotpRecovery {
    pub fn key<K>(key: K) -> Self
    where
        K: Into<String>
    {
        UpdateTotpRecovery {
            key: key.into(),
            body: UpdateTotpHash {
                key: None,
                regen: false
            }
        }
    }

    pub fn rename<K>(&mut self, key: K) -> &mut Self
    where
        K: Into<String>
    {
        self.body.key = Some(key.into());
        self
    }

    pub fn regen(&mut self, regen: bool) -> &mut Self {
        self.body.regen = regen;
        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<TotpRecovery>, RequestError> {
        if !rfs_lib::sec::authn::totp::recovery::key_valid(&self.key) {
            return Err(RequestError::Api(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("key")
            ))));
        }

        self.body.validate()?;

        let res = client.patch(format!("/api/user/totp/recovery/{}", self.key))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteTotpRecovery {
    key: String
}

impl DeleteTotpRecovery {
    pub fn key<K>(key: K) -> Self
    where
        K: Into<String>
    {
        DeleteTotpRecovery {
            key: key.into()
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        if !rfs_lib::sec::authn::totp::recovery::key_valid(&self.key) {
            return Err(RequestError::Api(ApiError::from((
                ApiErrorKind::ValidationFailed,
                Detail::with_key("key")
            ))));
        }

        let res = client.delete(format!("/api/user/totp/recovery/{}", self.key)).send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

