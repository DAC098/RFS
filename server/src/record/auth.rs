use super::ids;

pub struct Session {
    pub token: String,
    pub user_id: ids::UserId,
    pub dropped: bool,
    pub issued_on: String,
    pub expires: String,
    pub verified: bool,
}

pub struct CsrfToken {
    pub token: String,
    pub user_id: ids::UserId,
    pub issued_on: String,
    pub expires: String,
    pub path: String,
}

/*
pub mod policies {
    pub struct PolicyModel<Ability, Action, Resource = ()> {
        ability: Ability,
        action: Action,
        resource: Option<Resource>,
    }
}

pub mod Global {
    pub enum AuthAction {}

    pub enum UserAction {}

    pub enum Ability {
        Auth(AuthAction),
        User(UserAction),
    }
}

pub mod Resource {
    pub enum FSAction {}

    pub enum Ability {
        FS(FSAction),
    }
}

pub enum SubjectId {
    User(ids::UserId),
    Bot(ids::BotId),
}

pub enum ResourceId {
    User(ids::UserId),
    FS(ids::FSId),
    Storage(ids::StorageId),
}


pub struct Rbac {
    pub id: ids::RbacId,
    pub name: String,
    pub subjects: Vec<SubjectId>,
    pub policies: Vec<Policy>,
    pub created: String,
    pub updated: Option<String>,
    pub deleted: Option<String>,
}
*/
