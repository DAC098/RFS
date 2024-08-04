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
    uid: ids::FSUid,
}

impl RetrieveItem {
    pub fn uid(uid: ids::FSUid) -> Self {
        RetrieveItem { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Item>>, RequestError> {
        let res = client.get(format!("/api/fs/{}", self.uid)).send()?;

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
    last_id: Option<ids::FSUid>,
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
        I: Into<Option<ids::FSUid>>
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
    type Id = ids::FSUid;
    type Item = ItemMin;

    #[inline]
    fn get_last_id(item: &Self::Item) -> Option<Self::Id> {
        Some(match item {
            ItemMin::Root(root) => root.uid.clone(),
            ItemMin::Directory(dir) => dir.uid.clone(),
            ItemMin::File(file) => file.uid.clone(),
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
    uid: ids::FSUid,
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::FSUid>,
}

impl RetrieveContents {
    pub fn uid(uid: ids::FSUid) -> Self {
        RetrieveContents {
            uid,
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
        I: Into<Option<ids::FSUid>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<ItemMin>>, RequestError> {
        let mut builder = client.get(format!("/api/fs/{}/contents", self.uid));

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
    type Id = ids::FSUid;
    type Item = ItemMin;

    #[inline]
    fn get_last_id(item: &Self::Item) -> Option<Self::Id> {
        Some(match item {
            ItemMin::Root(root) => root.uid.clone(),
            ItemMin::Directory(dir) => dir.uid.clone(),
            ItemMin::File(file) => file.uid.clone(),
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
    uid: ids::FSUid
}

impl DownloadItem {
    pub fn uid(uid: ids::FSUid) -> Self {
        DownloadItem { uid }
    }

    pub fn send(&self, client: &ApiClient) -> Result<Response, RequestError> {
        let res = client.get(format!("/api/fs/{}/download", self.uid))
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateDir {
    parent: ids::FSUid,
    body: CreateDirBody
}

impl CreateDir {
    pub fn basename<B>(parent: ids::FSUid, basename: B) -> Self
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
        let res = client.post(format!("/api/fs/{}", self.parent))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct SendReadable {
    uid: ids::FSUid,
    basename: Option<String>,
    content_type: Option<mime::Mime>,
    content_length: Option<u64>,
    hash: Option<String>,
}

impl SendReadable {
    pub fn create<B>(parent: ids::FSUid, basename: B) -> SendReadable
    where
        B: Into<String>,
    {
        SendReadable {
            uid: parent,
            basename: Some(basename.into()),
            content_type: None,
            content_length: None,
            hash: None,
        }
    }

    pub fn update(uid: ids::FSUid) -> SendReadable {
        SendReadable {
            uid,
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
        let mut builder = client.put(format!("/api/fs/{}", self.uid))
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
    uid: ids::FSUid,
    body: UpdateMetadataBody
}

impl UpdateMetadata {
    pub fn uid(uid: ids::FSUid) -> Self {
        UpdateMetadata {
            uid,
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
        let res = client.patch(format!("/api/fs/{}", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteItem {
    uid: ids::FSUid
}

impl DeleteItem {
    pub fn uid(uid: ids::FSUid) -> Self {
        DeleteItem { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/api/fs/{}", self.uid)).send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
