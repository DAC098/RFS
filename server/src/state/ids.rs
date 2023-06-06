use lib::ids;
use snowcloud_cloud::error::Error;
use snowcloud_cloud::sync::MutexGenerator;
use snowcloud_cloud::wait::blocking_next_id;

use crate::net::error::{Result as NetResult, Error as NetError};

pub const START_TIME: u64 = 1685862000000;

pub type UserIdGenerator = MutexGenerator<ids::UserId>;
pub type StorageIdGenerator = MutexGenerator<ids::StorageId>;
pub type FSIdGenerator = MutexGenerator<ids::FSId>;

pub struct Ids {
    user: UserIdGenerator,
    storage: StorageIdGenerator,
    fs: FSIdGenerator,
}

impl Ids {
    pub fn new(primary: i64) -> Result<Self, Error> {
        Ok(Ids {
            user: MutexGenerator::new(START_TIME, primary)?,
            storage: MutexGenerator::new(START_TIME, primary)?,
            fs: MutexGenerator::new(START_TIME, primary)?
        })
    }

    pub fn user(&self) -> &UserIdGenerator {
        &self.user
    }

    pub fn wait_user_id(&self) -> NetResult<ids::UserId> {
        let Some(id) = blocking_next_id(&self.user, 5) else {
            return Err(NetError::new()
                .source("failed to generatoe user id. no more attempts"));
        };

        id.map_err(Into::into)
    }

    pub fn storage(&self) -> &StorageIdGenerator {
        &self.storage
    }

    pub fn wait_storage_id(&self) -> NetResult<ids::StorageId> {
        let Some(id) = blocking_next_id(&self.user, 5) else {
            return Err(NetError::new()
                .source("failed to generatoe user id. no more attempts"));
        };

        id.map_err(Into::into)
    }

    pub fn fs(&self) -> &FSIdGenerator {
        &self.fs
    }

    pub fn wait_fs_id(&self) -> NetResult<ids::FSId> {
        let Some(id) = blocking_next_id(&self.user, 5) else {
            return Err(NetError::new()
                .source("failed to generatoe user id. no more attempts"));
        };

        id.map_err(Into::into)
    }
}

impl std::fmt::Debug for Ids {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ids")
            .finish()
    }
}

