use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{
    Validator,
    Payload,
    ApiError,
    ApiErrorKind,
    Limit,
    Offset,
};
use crate::users::{
    CreateUser as CreateUserBody,
    UpdateUser as UpdateUserBody,
    User,
    ListItem,
};

pub mod groups;

pub struct QueryUsers {
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::UserId>,
}

impl QueryUsers {
    pub fn new() -> Self {
        QueryUsers {
            limit: None,
            offset: None,
            last_id: None,
        }
    }

    pub fn limit<L>(&mut self, limit: Limit) -> &mut Self
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
        I: Into<Option<ids::UserId>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<ListItem>>, RequestError> {
        let mut builder = client.get("/user");

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

pub struct RetrieveUser {
    id: ids::UserId
}

impl RetrieveUser {
    pub fn id(id: ids::UserId) -> Self {
        RetrieveUser { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<User>>, RequestError> {
        let res = client.get(format!("/user/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::UserNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            },
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateUser {
    body: CreateUserBody
}

impl CreateUser {
    pub fn username<U>(username: U) -> Self
    where
        U: Into<String>
    {
        CreateUser {
            body: CreateUserBody {
                username: username.into(),
                email: None
            }
        }
    }

    pub fn email<E>(&mut self, email: E) -> &mut Self
    where
        E: Into<String>
    {
        self.body.email = Some(email.into());
        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<User>, RequestError> {
        self.body.assert_ok()?;

        let res = client.post("/user")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateUser {
    id: ids::UserId,
    body: UpdateUserBody
}

impl UpdateUser {
    pub fn id(id: ids::UserId) -> Self {
        UpdateUser {
            id,
            body: UpdateUserBody {
                username: None,
                email: None
            }
        }
    }

    pub fn username<U>(&mut self, username: U) -> &mut Self
    where
        U: Into<String>
    {
        self.body.username = Some(username.into());
        self
    }

    pub fn email<E>(&mut self, email: Option<E>) -> &mut Self
    where
        E: Into<String>
    {
        self.body.email = Some(email.map(|v| v.into()));
        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<User>, RequestError> {
        self.body.assert_ok()?;

        let res = client.patch(format!("/user/{}", self.id.id()))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteUser {
    id: ids::UserId,
}

impl DeleteUser {
    pub fn id(id: ids::UserId) -> Self {
        DeleteUser { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/user/{}", self.id.id())).send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
