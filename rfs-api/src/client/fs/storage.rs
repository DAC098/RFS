use std::path::PathBuf;

use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::{ApiClient, iterate};
use crate::{
    Payload,
    ApiError,
    ApiErrorKind,
    Tags,
    Limit,
    Offset,
};
use crate::fs::{
    CreateStorage as CreateStorageBody,
    UpdateStorage as UpdateStorageBody,
    Storage,
    StorageMin,
    backend,
};

pub struct QueryStorage {
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::StorageId>,
}

impl QueryStorage {
    pub fn new() -> Self {
        QueryStorage {
            limit: None,
            offset: None,
            last_id: None,
        }
    }

    pub fn limit<L>(&mut self, limit: L) -> &mut Self
    where
        L: Into<Option<Limit>>
    {
        self.limit = limit.into();
        self
    }

    pub fn offset<O>(&mut self, offset: O) -> &mut Self
    where
        O: Into<Option<Offset>>
    {
        self.offset = offset.into();
        self
    }

    pub fn last_id<I>(&mut self, last_id: I) -> &mut Self
    where
        I: Into<Option<ids::StorageId>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<StorageMin>>, RequestError> {
        let mut builder = client.get("/api/fs/storage");

        if let Some(limit) = &self.limit {
            builder = builder.query(&[("limit", limit)]);
        }

        if let Some(last_id) = &self.last_id {
            builder = builder.query(&[("last_id", last_id)]);
        } else if let Some(offset) = &self.offset {
            builder = builder.query(&[("offset", offset)]);
        }

        let res = builder.send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

impl iterate::Pageable for QueryStorage {
    type Id = ids::StorageId;
    type Item = StorageMin;

    #[inline]
    fn get_last_id(item: &Self::Item) -> Option<Self::Id> {
        Some(item.id.clone())
    }

    #[inline]
    fn set_limit(&mut self, limit: Option<Limit>) {
        self.limit(limit);
    }

    #[inline]
    fn set_last_id(&mut self, id: Option<Self::Id>) {
        self.last_id(id);
    }

    #[inline]
    fn send(&self, client: &ApiClient) -> Result<Payload<Vec<Self::Item>>, RequestError> {
        QueryStorage::send(self, client)
    }
}

pub struct RetrieveStorage {
    id: ids::StorageId
}

impl RetrieveStorage {
    pub fn id(id: ids::StorageId) -> Self {
        RetrieveStorage { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Storage>>, RequestError> {
        let res = client.get(format!("/api/fs/storage/{}", self.id.id())).send()?;

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
                backend: backend::CreateConfig::Local {
                    path: path.into()
                },
                tags: Tags::new()
            }
        }
    }

    pub fn comment<C>(&mut self, _comment: C) -> &mut Self
    where
        C: Into<String>
    {
        self
    }

    pub fn add_tag<T, V>(&mut self, tag: T, value: Option<V>) -> &mut Self
    where
        T: Into<String>,
        V: Into<String>,
    {
        self.body.tags.insert(tag.into(), value.map(|v| v.into()));
        self
    }

    pub fn add_iter_tags<I, T, V>(&mut self, iter: I) -> &mut Self
    where
        T: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (T, Option<V>)>
    {
        for (k, v) in iter {
            self.body.tags.insert(k.into(), v.map(|v| v.into()));
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Storage>, RequestError> {
        let res = client.post("/api/fs/storage")
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
                backend: None,
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

    pub fn add_iter_tags<I, T, V>(&mut self, iter: I) -> &mut Self
    where
        T: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (T, Option<V>)>
    {
        if let Some(tags) = &mut self.body.tags {
            for (k, v) in iter {
                tags.insert(k.into(), v.map(|v| v.into()));
            }
        } else {
            let mut tags = Tags::new();

            for (k, v) in iter {
                tags.insert(k.into(), v.map(|v| v.into()));
            }

            self.body.tags = Some(tags);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Storage>, RequestError> {
        let res = client.put(format!("/api/fs/storage/{}", self.id.id()))
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
        let res = client.delete(format!("/api/fs/storage/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
