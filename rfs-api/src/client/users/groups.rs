use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::{ApiClient, iterate};
use crate::{
    Payload,
    Validator,
    ApiError,
    ApiErrorKind,
    Limit,
    Offset
};
use crate::users::groups::{
    CreateGroup as CreateGroupBody,
    UpdateGroup as UpdateGroupBody,
    AddUsers as AddUsersBody,
    DropUsers as DropUsersBody,
    Group,
    GroupUser,
    ListItem,
};

pub struct QueryGroups {
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::GroupUid>,
}

impl QueryGroups {
    pub fn new() -> Self {
        QueryGroups {
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
        I: Into<Option<ids::GroupUid>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<ListItem>>, RequestError> {
        let mut builder= client.get("/api/user/group");

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
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

impl iterate::Pageable for QueryGroups {
    type Id = ids::GroupUid;
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

pub struct QueryGroupUsers {
    uid: ids::GroupUid,
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::UserUid>,
}

impl QueryGroupUsers {
    pub fn uid(uid: ids::GroupUid) -> Self {
        QueryGroupUsers {
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
        I: Into<Option<ids::UserUid>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<GroupUser>>, RequestError> {
        let mut builder = client.get(format!("/api/user/group/{}/users", self.uid));

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

impl iterate::Pageable for QueryGroupUsers {
    type Id = ids::UserUid;
    type Item = GroupUser;

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

pub struct RetrieveGroup {
    uid: ids::GroupUid,
}

impl RetrieveGroup {
    pub fn uid(uid: ids::GroupUid) -> Self {
        RetrieveGroup { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Group>>, RequestError> {
        let res = client.get(format!("/api/user/group/{}", self.uid)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::GroupNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            },
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct CreateGroup {
    body: CreateGroupBody
}

impl CreateGroup {
    pub fn name<N>(name: N) -> Self
    where
        N: Into<String>
    {
        CreateGroup {
            body: CreateGroupBody {
                name: name.into()
            }
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Group>, RequestError> {
        self.body.assert_ok()?;

        let res = client.post("/api/user/group")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateGroup {
    uid: ids::GroupUid,
    body: UpdateGroupBody,
}

impl UpdateGroup {
    pub fn uid<N>(uid: ids::GroupUid, name: N) -> Self
    where
        N: Into<String>
    {
        UpdateGroup {
            uid,
            body: UpdateGroupBody {
                name: name.into()
            }
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Group>, RequestError> {
        self.body.assert_ok()?;

        let res = client.patch(format!("/user/group/{}", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct DeleteGroup {
    uid: ids::GroupUid,
}

impl DeleteGroup {
    pub fn uid(uid: ids::GroupUid) -> Self {
        DeleteGroup { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Group>, RequestError> {
        let res = client.delete(format!("/api/user/group/{}", self.uid)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct AddUsers {
    uid: ids::GroupUid,
    body: AddUsersBody
}

impl AddUsers {
    pub fn uid(uid: ids::GroupUid) -> Self {
        AddUsers {
            uid,
            body: AddUsersBody {
                uids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, user_uid: ids::UserUid) -> &mut Self {
        self.body.uids.push(user_uid);
        self
    }

    pub fn add_iter<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserUid>
    {
        for id in iter {
            self.body.uids.push(id);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.post(format!("/api/user/group/{}/users", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DropUsers {
    uid: ids::GroupUid,
    body: DropUsersBody
}

impl DropUsers {
    pub fn uid(uid: ids::GroupUid) -> Self {
        DropUsers {
            uid,
            body: DropUsersBody {
                uids: Vec::new(),
            }
        }
    }

    pub fn add_id(&mut self, user_uid: ids::UserUid) -> &mut Self {
        self.body.uids.push(user_uid);
        self
    }

    pub fn add_iter<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserUid>
    {
        for id in iter {
            self.body.uids.push(id);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/api/user/group/{}/users", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

