use super::ids::{ListenerId, UserId};

pub enum Event {
    UserCreated,
    UserUpdated,
    UserDeleted,
    FSCreated,
    FSUpdated,
    FSDeleted,
}

pub struct Listener {
    pub id: ListenerId,
    pub name: String,
    pub secret: String,
    pub endpoint: String,
    pub events: Vec<Event>,
}
