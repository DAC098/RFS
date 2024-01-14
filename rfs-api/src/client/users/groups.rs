use rfs_lib::ids;

use crate::client::error::RequestError;
use crate::client::ApiClient;
use crate::{Payload, Validator, ApiError, ApiErrorKind};
use crate::users::groups::{
    CreateGroup as CreateGroupBody,
    UpdateGroup as UpdateGroupBody,
    AddUsers as AddUsersBody,
    DropUsers as DropUsersBody,
    Group,
    GroupUser,
    ListItem,
};

pub struct QueryGroups {}

impl QueryGroups {
    pub fn new() -> Self {
        QueryGroups {}
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Vec<ListItem>>, RequestError> {
        let res = client.get("/user/group").send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct QueryGroupUsers {
    id: ids::GroupId
}

impl QueryGroupUsers {
    pub fn id(id: ids::GroupId) -> Self {
        QueryGroupUsers { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Vec<GroupUser>>>, RequestError> {
        let res = client.get(format!("/user/group/{}/users", self.id)).send()?;

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

pub struct RetrieveGroup {
    id: ids::GroupId,
}

impl RetrieveGroup {
    pub fn id(id: ids::GroupId) -> Self {
        RetrieveGroup { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Option<Payload<Group>>, RequestError> {
        let res = client.get(format!("/user/group/{}", self.id)).send()?;

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

        let res = client.post("/user/group")
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::CREATED => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct UpdateGroup {
    id: ids::GroupId,
    body: UpdateGroupBody,
}

impl UpdateGroup {
    pub fn id<N>(id: ids::GroupId, name: N) -> Self
    where
        N: Into<String>
    {
        UpdateGroup {
            id,
            body: UpdateGroupBody {
                name: name.into()
            }
        }
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Group>, RequestError> {
        self.body.assert_ok()?;

        let res = client.patch(format!("/user/group/{}", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct DeleteGroup {
    id: ids::GroupId,
}

impl DeleteGroup {
    pub fn id(id: ids::GroupId) -> Self {
        DeleteGroup { id }
    }

    pub fn send(self, client: &ApiClient) -> Result<Payload<Group>, RequestError> {
        let res = client.delete(format!("/user/group/{}", self.id)).send()?;

        match res.status() {
            reqwest::StatusCode::OK => Ok(res.json()?),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

pub struct AddUsers {
    id: ids::GroupId,
    body: AddUsersBody
}

impl AddUsers {
    pub fn id(id: ids::GroupId) -> Self {
        AddUsers {
            id,
            body: AddUsersBody {
                ids: Vec::new()
            }
        }
    }

    pub fn add_id(&mut self, user_id: ids::UserId) -> &mut Self {
        self.body.ids.push(user_id);
        self
    }

    pub fn add_iter<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserId>
    {
        for id in iter {
            self.body.ids.push(id);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.post(format!("/user/group/{}/users", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?))
        }
    }
}

pub struct DropUsers {
    id: ids::GroupId,
    body: DropUsersBody
}

impl DropUsers {
    pub fn id(id: ids::GroupId) -> Self {
        DropUsers {
            id,
            body: DropUsersBody {
                ids: Vec::new(),
            }
        }
    }

    pub fn add_id(&mut self, user_id: ids::UserId) -> &mut Self {
        self.body.ids.push(user_id);
        self
    }

    pub fn add_iter<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ids::UserId>
    {
        for id in iter {
            self.body.ids.push(id);
        }

        self
    }

    pub fn send(self, client: &ApiClient) -> Result<(), RequestError> {
        let res = client.delete(format!("/user/group/{}/users", self.id))
            .json(&self.body)
            .send()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()),
            _ => Err(RequestError::Api(res.json()?)),
        }
    }
}

