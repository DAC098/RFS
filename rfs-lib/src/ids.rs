use std::fmt::Debug;

use snowcloud_flake::i64::SingleIdFlake;

pub const START_TIME: u64 = 1685862000000;

pub type UserId = SingleIdFlake<43, 8, 12>;
pub type GroupId = i64;
pub type RoleId = i64;
pub type FSId = SingleIdFlake<43, 8, 12>;
pub type StorageId = SingleIdFlake<43, 8, 12>;
pub type BotId = SingleIdFlake<43, 8, 12>;
pub type ListenerId = SingleIdFlake<43, 8, 12>;

pub fn from_pg<V, T>(value: V) -> T
where
    T: TryFrom<V>,
    T::Error: Debug,
{
    TryFrom::try_from(value)
        .expect("failed to retrieve id from database")
}

#[inline]
pub fn user_id_from_pg(value: i64) -> UserId {
    from_pg(value)
}

#[inline]
pub fn fs_id_from_pg(value: i64) -> FSId {
    from_pg(value)
}

#[inline]
pub fn storage_id_from_pg(value: i64) -> StorageId {
    from_pg(value)
}

#[inline]
pub fn bot_id_from_pg(value: i64) -> BotId {
    from_pg(value)
}

#[inline]
pub fn listener_id_from_pg(value: i64) -> ListenerId {
    from_pg(value)
}
