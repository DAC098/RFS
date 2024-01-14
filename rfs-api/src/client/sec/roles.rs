use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{Payload, Validator, ApiError, ApiErrorKind};
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

pub struct QueryRoles {}

impl QueryRoles {
    pub fn new() -> Self {
        QueryRoles {}
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Vec<RoleListItem>>, RequestError> {
        let res = client.get("/sec/roles").send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct RetrieveRole {
    id: ids::RoleId
}

impl RetrieveRole {
    pub fn id(id: ids::RoleId) -> Self {
        RetrieveRole { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Vec<Role>>>, RequestError> {
        let res = client.get(format!("/sec/roles/{}", self.id)).send()?;

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
    id: ids::RoleId
}

impl QueryRoleUsers {
    pub fn id(id: ids::RoleId) -> Self {
        QueryRoleUsers { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Vec<RoleUser>>>, RequestError> {
        let res = client.get(format!("/sec/roles/{}/users", self.id)).send()?;

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

pub struct QueryRoleGroups {
    id: ids::RoleId
}

impl QueryRoleGroups {
    pub fn id(id: ids::RoleId) -> Self {
        QueryRoleGroups { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Vec<RoleGroup>>>, RequestError> {
        let res = client.get(format!("/sec/roles/{}/groups", self.id)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(Some(res.json()?)),
            reqwest::StatusCode::NOT_FOUND => {
                let err: ApiError = res.json()?;

                if *err.kind() == ApiErrorKind::RoleNotFound {
                    return Ok(None);
                }

                Err(RequestError::Api(err))
            }
            _ => Err(RequestError::Api(res.json()?))
        }
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

        let res = client.post("/sec/roles")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateRole {
    id: ids::RoleId,
    body: UpdateRoleBody
}

impl UpdateRole {
    pub fn id(id: ids::RoleId) -> Self {
        UpdateRole {
            id,
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

        let res = client.patch(format!("/sec/roles/{}", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DeleteRole {
    id: ids::RoleId,
}

impl DeleteRole {
    pub fn id(id: ids::RoleId) -> Self {
        DeleteRole { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/sec/roles/{}", self.id)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct AddRoleUsers {
    id: ids::RoleId,
    body: AddRoleUserBody
}

impl AddRoleUsers {
    pub fn id(id: ids::RoleId) -> Self {
        AddRoleUsers {
            id,
            body: AddRoleUserBody {
                ids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, id: ids::UserId) -> &mut Self {
        self.body.ids.push(id);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserId>
    {
        for item in iter {
            self.body.ids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.post(format!("/sec/roles/{}/users", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DropRoleUsers {
    id: ids::RoleId,
    body: DropRoleUserBody
}

impl DropRoleUsers {
    pub fn id(id: ids::RoleId) -> Self {
        DropRoleUsers {
            id,
            body: DropRoleUserBody {
                ids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, id: ids::UserId) -> &mut Self {
        self.body.ids.push(id);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserId>
    {
        for item in iter {
            self.body.ids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.delete(format!("/sec/roles/{}/users", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct AddRoleGroups {
    id: ids::RoleId,
    body: AddRoleGroupBody
}

impl AddRoleGroups {
    pub fn id(id: ids::RoleId) -> Self {
        AddRoleGroups {
            id,
            body: AddRoleGroupBody {
                ids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, id: ids::GroupId) -> &mut Self {
        self.body.ids.push(id);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::GroupId>
    {
        for item in iter {
            self.body.ids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.post(format!("/sec/roles/{}/groups", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DropRoleGroups {
    id: ids::RoleId,
    body: DropRoleGroupBody
}

impl DropRoleGroups {
    pub fn id(id: ids::RoleId) -> Self {
        DropRoleGroups {
            id,
            body: DropRoleGroupBody {
                ids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, id: ids::GroupId) -> &mut Self {
        self.body.ids.push(id);
        self
    }

    pub fn add_iter_id<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::GroupId>
    {
        for item in iter {
            self.body.ids.push(item);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        self.body.assert_ok()?;

        let res = client.delete(format!("/sec/roles/{}/groups", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}
