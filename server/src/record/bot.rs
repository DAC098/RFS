use super::ids::{BotId, UserId};

pub struct Bot {
    pub id: BotId,
    pub name: String,
    pub users_id: UserId,
    pub secret: String,
    pub created: String,
    pub updated: Option<String>,
    pub deleted: Option<String>,
}
