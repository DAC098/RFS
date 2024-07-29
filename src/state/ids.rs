use rfs_lib::ids;
use snowcloud_cloud::error::Error;
use snowcloud_cloud::sync::MutexGenerator;
use snowcloud_cloud::wait::blocking_next_id;

use crate::error::ApiResult;
use crate::error::api::Context;

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

    pub fn wait_user_id(&self) -> ApiResult<ids::UserId> {
        blocking_next_id(&self.user, 5)
            .context("failed to generate user id. no more attempts")?
            .context("failed to generate user id")
    }

    pub fn wait_storage_id(&self) -> ApiResult<ids::StorageId> {
        blocking_next_id(&self.storage, 5)
            .context("failed to generate storage id. no more attempts")?
            .context("failed to generate storage id")
    }

    pub fn wait_fs_id(&self) -> ApiResult<ids::FSId> {
        blocking_next_id(&self.fs, 5)
            .context("failed to generate fs id. no more attempts")?
            .context("failed to generate fs id")
    }
}

impl std::fmt::Debug for Ids {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ids")
            .finish()
    }
}

