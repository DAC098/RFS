use rfs_lib::ids;
use reqwest::blocking::Body;

use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{Payload, ApiError, ApiErrorKind, Tags};
use crate::fs::{
    //CreateItem as CreateItemBody,
    CreateDir as CreateDirBody,
    UpdateMetadata as UpdateMetadataBody,
    //ListItem,
    Item
};

pub mod storage;

pub struct RetrieveItem {
    id: ids::FSId
}

impl RetrieveItem {
    pub fn id(id: ids::FSId) -> Self {
        RetrieveItem { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Item>>, RequestError> {
        let res = client.get(format!("/fs/{}", self.id.id())).send()?;

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

    pub fn send(self, client: &ApiClient) -> Result<Payload<Item>, RequestError> {
        //self.body.assert_ok()?;

        let res = client.post(format!("/fs/{}", self.parent.id()))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct SendReadable<R>
where
    R: std::io::Read + Send + 'static
{
    parent: ids::FSId,
    reader: R,
    basename: String,
    content_type: Option<mime::Mime>,
}

impl<R> SendReadable<R>
where
    R: std::io::Read + Send + 'static
{
    pub fn reader<T, B>(parent: ids::FSId, basename: B, reader: T) -> SendReadable<T>
    where
        B: Into<String>,
        T: std::io::Read + Send
    {
        SendReadable {
            parent,
            reader,
            basename: basename.into(),
            content_type: None,
        }
    }

    pub fn content_type<M>(&mut self, content_type: M) -> Result<&mut Self, mime::FromStrError>
    where
        M: AsRef<str>
    {
        self.content_type = Some(content_type.as_ref().parse()?);

        Ok(self)
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Item>, RequestError> {
        let content_type = self.content_type.unwrap_or(mime::APPLICATION_OCTET_STREAM);
        let res = client.put(format!("/fs/{}", self.parent.id()))
            .header("x-basename", self.basename)
            .header("content-type", content_type.to_string())
            .body(Body::new(self.reader))
            .send()?;

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

    pub fn send(self, client: &ApiClient) -> Result<Payload<Item>, RequestError> {
        let res = client.patch(format!("/fs/{}", self.id.id()))
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
        let res = client.delete(format!("/fs/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
