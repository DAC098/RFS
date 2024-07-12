use rfs_lib::query::{Limit, Offset};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PaginationQuery<T> {
    #[serde(default)]
    pub limit: Limit,

    #[serde(default)]
    pub offset: Offset,

    pub last_id: Option<T>,
}
