use rfs_lib::ids;
use reqwest::blocking::Body;
use reqwest::blocking::Response;

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
    CreateDir as CreateDirBody,
    UpdateMetadata as UpdateMetadataBody,
    Item,
    ItemMin,
};

pub mod storage;

pub struct RetrieveItem {
    id: ids::FSId,
}

impl RetrieveItem {
    pub fn id(id: ids::FSId) -> Self {
        RetrieveItem { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Item>>, RequestError> {
        let res = client.get(format!("/api/fs/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::FileNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            },
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct RetrieveRoots {
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::FSId>,
}

impl RetrieveRoots {
    pub fn new() -> Self {
        RetrieveRoots {
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
        I: Into<Option<ids::FSId>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<ItemMin>>, RequestError> {
        let mut builder = client.get("/api/fs");

        if let Some(limit) = &self.limit {
            builder = builder.query(&[("limit", limit)]);
        }

        if let Some(last_id) = &self.last_id {
            builder = builder.query(&[("last_id", last_id)]);
        } else if let  Some(offset) = &self.offset {
            builder = builder.query(&[("offset", offset)]);
        }

        let res = builder.send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

impl iterate::Pageable for RetrieveRoots {
    type Id = ids::FSId;
    type Item = ItemMin;

    #[inline]
    fn get_last_id(item: &Self::Item) -> Option<Self::Id> {
        Some(match item {
            ItemMin::Root(root) => root.id.clone(),
            ItemMin::Directory(dir) => dir.id.clone(),
            ItemMin::File(file) => file.id.clone(),
        })
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
        self.send(client)
    }
}

pub struct RetrieveContents {
    id: ids::FSId,
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::FSId>,
}

impl RetrieveContents {
    pub fn id(id: ids::FSId) -> Self {
        RetrieveContents {
            id,
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
        I: Into<Option<ids::FSId>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<ItemMin>>, RequestError> {
        let mut builder = client.get(format!("/api/fs/{}/contents", self.id.id()));

        if let Some(limit) = &self.limit {
            builder = builder.query(&[("limit", limit)]);
        }

        if let Some(last_id) = &self.last_id {
            builder = builder.query(&[("last_id", last_id)]);
        } else if let  Some(offset) = &self.offset {
            builder = builder.query(&[("offset", offset)]);
        }

        let res = builder.send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

impl iterate::Pageable for RetrieveContents {
    type Id = ids::FSId;
    type Item = ItemMin;

    #[inline]
    fn get_last_id(item: &Self::Item) -> Option<Self::Id> {
        Some(match item {
            ItemMin::Root(root) => root.id.clone(),
            ItemMin::Directory(dir) => dir.id.clone(),
            ItemMin::File(file) => file.id.clone(),
        })
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
        self.send(client)
    }
}

pub struct DownloadItem {
    id: ids::FSId
}

impl DownloadItem {
    pub fn id(id: ids::FSId) -> Self {
        DownloadItem { id }
    }

    pub fn send(&self, client: &ApiClient) -> Result<Response, RequestError> {
        let res = client.get(format!("/api/fs/{}/download", self.id.id()))
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateDir {
    parent: ids::FSId,
    body: CreateDirBody
}

impl CreateDir {
    pub fn basename<B>(parent: ids::FSId, basename: B) -> Self
    where
        B: Into<String>
    {
        CreateDir {
            parent,
            body: CreateDirBody {
                basename: basename.into(),
                tags: None,
                comment: None
            }
        }
    }

    pub fn comment<C>(&mut self, comment: C) -> &mut Self
    where
        C: Into<String>
    {
        self.body.comment = Some(comment.into());
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

    pub fn send(self, client: &ApiClient) -> Result<Payload<Item>, RequestError> {
        //self.body.assert_ok()?;

        let res = client.post(format!("/api/fs/{}", self.parent.id()))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct SendReadable {
    id: ids::FSId,
    basename: Option<String>,
    content_type: Option<mime::Mime>,
    content_length: Option<u64>,
    hash: Option<String>,
}

impl SendReadable {
    pub fn create<B>(parent: ids::FSId, basename: B) -> SendReadable
    where
        B: Into<String>,
    {
        SendReadable {
            id: parent,
            basename: Some(basename.into()),
            content_type: None,
            content_length: None,
            hash: None,
        }
    }

    pub fn update(id: ids::FSId) -> SendReadable {
        SendReadable {
            id,
            basename: None,
            content_type: None,
            content_length: None,
            hash: None,
        }
    }

    pub fn content_type(&mut self, mime: mime::Mime) -> &mut Self {
        self.content_type = Some(mime);
        self
    }

    pub fn content_length(&mut self, length: u64) -> &mut Self {
        self.content_length = Some(length);
        self
    }

    pub fn hash<H,V>(&mut self, hash_type: H, hash_value: V) -> &mut Self
    where
        H: Into<String>,
        V: Into<String>,
    {
        let hash_type = hash_type.into();
        let hash_value = hash_value.into();

        self.hash = Some(format!("{hash_type}:{hash_value}"));
        self
    }

    pub fn send<R>(self, client: &ApiClient, reader: R) -> Result<Payload<Item>, RequestError>
    where
        R: std::io::Read + Send + 'static
    {
        let content_type = self.content_type.unwrap_or(mime::APPLICATION_OCTET_STREAM);
        let mut builder = client.put(format!("/api/fs/{}", self.id.id()))
            .header("content-type", content_type.to_string());

        if let Some(length) = self.content_length {
            builder = builder.header("content-length", length);
        }

        if let Some(basename) = self.basename {
            builder = builder.header("x-basename", basename);
        }

        if let Some(hash) = self.hash {
            builder = builder.header("x-hash", hash);
        }

        let res = builder.body(Body::new(reader)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateMetadata {
    id: ids::FSId,
    body: UpdateMetadataBody
}

impl UpdateMetadata {
    pub fn id(id: ids::FSId) -> Self {
        UpdateMetadata {
            id,
            body: UpdateMetadataBody {
                tags: None,
                comment: None
            }
        }
    }

    pub fn comment<C>(&mut self, comment: C) -> &mut Self
    where
        C: Into<String>
    {
        self.body.comment = Some(comment.into());
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

    pub fn send(self, client: &ApiClient) -> Result<Payload<Item>, RequestError> {
        let res = client.patch(format!("/api/fs/{}", self.id.id()))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteItem {
    id: ids::FSId
}

impl DeleteItem {
    pub fn id(id: ids::FSId) -> Self {
        DeleteItem { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/api/fs/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
