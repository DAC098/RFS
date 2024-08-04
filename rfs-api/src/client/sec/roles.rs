use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::{ApiClient, iterate};
use crate::{
    Payload,
    Validator,
    ApiError,
    ApiErrorKind,
    Limit,
    Offset,
};
use crate::sec::roles::{
    CreateRole as CreateRoleBody,
    UpdateRole as UpdateRoleBody,
    AddRoleGroup as AddRoleGroupBody,
    AddRoleUser as AddRoleUserBody,
    DropRoleGroup as DropRoleGroupBody,
    DropRoleUser as DropRoleUserBody,
    Role,
    RoleListItem,
    RoleGroup,
    RoleUser,
    Permission
};

pub struct QueryRoles {
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::RoleUid>,
}

impl QueryRoles {
    pub fn new() -> Self {
        QueryRoles {
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
        I: Into<Option<ids::RoleUid>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<RoleListItem>>, RequestError> {
        let mut builder = client.get("/api/sec/roles");

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

impl iterate::Pageable for QueryRoles {
    type Id = ids::RoleUid;
    type Item = RoleListItem;

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

pub struct RetrieveRole {
    uid: ids::RoleUid
}

impl RetrieveRole {
    pub fn uid(uid: ids::RoleUid) -> Self {
        RetrieveRole { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Role>>, RequestError> {
        let res = client.get(format!("/api/sec/roles/{}", self.uid)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::RoleNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            },
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct QueryRoleUsers {
    uid: ids::RoleUid,
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::UserUid>,
}

impl QueryRoleUsers {
    pub fn uid(uid: ids::RoleUid) -> Self {
        QueryRoleUsers {
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

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<RoleUser>>, RequestError> {
        let mut builder = client.get(format!("/api/sec/roles/{}/users", self.uid));

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

impl iterate::Pageable for QueryRoleUsers {
    type Id = ids::UserUid;
    type Item = RoleUser;

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

pub struct QueryRoleGroups {
    uid: ids::RoleUid,
    limit: Option<Limit>,
    offset: Option<Offset>,
    last_id: Option<ids::GroupUid>,
}

impl QueryRoleGroups {
    pub fn uid(uid: ids::RoleUid) -> Self {
        QueryRoleGroups {
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
        I: Into<Option<ids::GroupUid>>
    {
        self.last_id = last_id.into();
        self
    }

    pub fn send(&self, client: &ApiClient) -> Result<Payload<Vec<RoleGroup>>, RequestError> {
        let mut builder = client.get(format!("/api/sec/roles/{}/groups", self.uid));

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

impl iterate::Pageable for QueryRoleGroups {
    type Id = ids::GroupUid;
    type Item = RoleGroup;

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

pub struct CreateRole {
    body: CreateRoleBody
}

impl CreateRole {
    pub fn name<N>(name: N) -> Self
    where
        N: Into<String>
    {
        CreateRole {
            body: CreateRoleBody {
                name: name.into(),
                permissions: Vec::new(),
            }
        }
    }

    pub fn add_permission<P>(&mut self, permission: P) -> &mut Self
    where
        P: Into<Permission>
    {
        self.body.permissions.push(permission.into());
        self
    }

    pub fn add_iter_permission<I, P>(&mut self, iter: I) -> &mut Self
    where
        P: Into<Permission>,
        I: IntoIterator<Item = P>
    {
        for item in iter {
            self.body.permissions.push(item.into());
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Role>, RequestError> {
        self.body.assert_ok()?;

        let res = client.post("/api/sec/roles")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateRole {
    uid: ids::RoleUid,
    body: UpdateRoleBody
}

impl UpdateRole {
    pub fn uid(uid: ids::RoleUid) -> Self {
        UpdateRole {
            uid,
            body: UpdateRoleBody {
                name: None,
                permissions: None
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

    pub fn add_permission<P>(&mut self, permission: P) -> &mut Self
    where
        P: Into<Permission>
    {
        if let Some(permissions) = &mut self.body.permissions {
            permissions.push(permission.into());
        } else {
            self.body.permissions = Some(vec![permission.into()])
        }

        self
    }

    pub fn add_iter_permissions<I, P>(&mut self, iter: I) -> &mut Self
    where
        P: Into<Permission>,
        I: IntoIterator<Item = P>
    {
        if let Some(permissions) = &mut self.body.permissions {
            for item in iter {
                permissions.push(item.into());
            }
        } else {
            self.body.permissions = Some(Vec::from_iter(iter.into_iter().map(|v| v.into())));
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Role>, RequestError> {
        self.body.assert_ok()?;

        let res = client.patch(format!("/api/sec/roles/{}", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteRole {
    uid: ids::RoleUid,
}

impl DeleteRole {
    pub fn uid(uid: ids::RoleUid) -> Self {
        DeleteRole { uid }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/api/sec/roles/{}", self.uid)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct AddRoleUsers {
    uid: ids::RoleUid,
    body: AddRoleUserBody
}

impl AddRoleUsers {
    pub fn uid(uid: ids::RoleUid) -> Self {
        AddRoleUsers {
            uid,
            body: AddRoleUserBody {
                uids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, uid: ids::UserUid) -> &mut Self {
        self.body.uids.push(uid);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserUid>
    {
        for item in iter {
            self.body.uids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.post(format!("/api/sec/roles/{}/users", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DropRoleUsers {
    uid: ids::RoleUid,
    body: DropRoleUserBody
}

impl DropRoleUsers {
    pub fn uid(uid: ids::RoleUid) -> Self {
        DropRoleUsers {
            uid,
            body: DropRoleUserBody {
                uids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, uid: ids::UserUid) -> &mut Self {
        self.body.uids.push(uid);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserUid>
    {
        for item in iter {
            self.body.uids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.delete(format!("/api/sec/roles/{}/users", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct AddRoleGroups {
    uid: ids::RoleUid,
    body: AddRoleGroupBody
}

impl AddRoleGroups {
    pub fn uid(uid: ids::RoleUid) -> Self {
        AddRoleGroups {
            uid,
            body: AddRoleGroupBody {
                uids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, id: ids::GroupUid) -> &mut Self {
        self.body.uids.push(id);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::GroupUid>
    {
        for item in iter {
            self.body.uids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.post(format!("/api/sec/roles/{}/groups", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DropRoleGroups {
    uid: ids::RoleUid,
    body: DropRoleGroupBody
}

impl DropRoleGroups {
    pub fn uid(uid: ids::RoleUid) -> Self {
        DropRoleGroups {
            uid,
            body: DropRoleGroupBody {
                uids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, uid: ids::GroupUid) -> &mut Self {
        self.body.uids.push(uid);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::GroupUid>
    {
        for item in iter {
            self.body.uids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.delete(format!("/api/sec/roles/{}/groups", self.uid))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
