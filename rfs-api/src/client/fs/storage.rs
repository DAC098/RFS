use std::path::PathBuf;

use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{Payload, ApiError, ApiErrorKind, Tags};
use crate::fs::storage::{
    CreateStorage as CreateStorageBody,
    CreateStorageType,
    UpdateStorage as UpdateStorageBody,
    StorageItem,
    StorageListItem,
};

pub struct QueryStorage {}

impl QueryStorage {
    pub fn new() -> Self {
        QueryStorage {}
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Vec<StorageListItem>>, RequestError> {
        let res = client.get("/storage").send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct RetrieveStorage {
    id: ids::StorageId
}

impl RetrieveStorage {
    pub fn id(id: ids::StorageId) -> Self {
        RetrieveStorage { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<StorageItem>>, RequestError> {
        let res = client.get(format!("/storage/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::StorageNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            }
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateStorage {
    body: CreateStorageBody
}

impl CreateStorage {
    pub fn local<N, P>(name: N, path: P) -> Self
    where
        N: Into<String>,
        P: Into<PathBuf>,
    {
        CreateStorage {
            body: CreateStorageBody {
                name: name.into(),
                type_: CreateStorageType::Local {
                    path: path.into()
                },
                tags: Tags::new()
            }
        }
    }

    pub fn add_tag<T, V>(&mut self, tag: T, value: Option<V>) -> &mut Self
    where
        T: Into<String>,
        V: Into<String>,
    {
        self.body.tags.insert(tag.into(), value.map(|v| v.into()));
        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<StorageItem>, RequestError> {
        let res = client.post("/storage")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateStorage {
    id: ids::StorageId,
    body: UpdateStorageBody
}

impl UpdateStorage {
    pub fn local(id: ids::StorageId) -> Self {
        UpdateStorage {
            id,
            body: UpdateStorageBody {
                name: None,
                type_: None,
                tags: None
            }
        }
    }

    pub fn name<N>(&mut self, name: N) -> &mut Self
    where
        N: Into<String>
    {
        self.body.name = Some(name.into());
        self
    }

    pub fn add_tag<T, V>(&mut self, tag: T, value: Option<V>) -> &mut Self
    where
        T: Into<String>,
        V: Into<String>,
    {
        if let Some(tags) = &mut self.body.tags {
            tags.insert(tag.into(), value.map(|v| v.into()));
        } else {
            self.body.tags = Some(Tags::from([(tag.into(), value.map(|v| v.into()))]));
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<StorageItem>, RequestError> {
        let res = client.put(format!("/storage/{}", self.id.id()))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteStorage {
    id: ids::StorageId
}

impl DeleteStorage {
    pub fn id(id: ids::StorageId) -> Self {
        DeleteStorage { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/storage/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
