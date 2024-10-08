use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::{ApiClient, iterate};
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
pub mod password;
pub mod totp;

pub struct QueryUsers {
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::UserUid>,
}

impl QueryUsers {
    pub fn new() -> Self {
        QueryUsers {
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
        I: Into<Option<ids::UserUid>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<ListItem>>, RequestError> {
        let mut builder = client.get("/api/user");

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

impl iterate::Pageable for QueryUsers {
    type Id = ids::UserUid;
    type Item = ListItem;

    #[inline]
    fn get_last_id(item: &Self::Item) -> Option<Self::Id> {
        Some(item.uid.clone())
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

pub struct RetrieveUser {
    uid: ids::UserUid
}

impl RetrieveUser {
    pub fn uid(uid: ids::UserUid) -> Self {
        RetrieveUser { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<User>>, RequestError> {
        let res = client.get(format!("/api/user/{}", self.uid)).send()?;

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
    pub fn username<U, P>(username: U, password: P) -> Self
    where
        U: Into<String>,
        P: Into<String>
    {
        CreateUser {
            body: CreateUserBody {
                username: username.into(),
                password: password.into(),
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

        let res = client.post("/api/user")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateUser {
    uid: ids::UserUid,
    body: UpdateUserBody
}

impl UpdateUser {
    pub fn uid(uid: ids::UserUid) -> Self {
        UpdateUser {
            uid,
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

        let res = client.patch(format!("/api/user/{}", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteUser {
    uid: ids::UserUid,
}

impl DeleteUser {
    pub fn uid(uid: ids::UserUid) -> Self {
        DeleteUser { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/api/user/{}", self.uid)).send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
