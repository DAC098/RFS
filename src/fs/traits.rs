use std::path::PathBuf;

use rfs_lib::ids;
use chrono::{DateTime, Utc};

pub trait Common {
    fn id(&self) -> &ids::FSId;
    fn parent(&self) -> Option<&ids::FSId>;
    fn user_id(&self) -> &ids::UserId;

    fn full_path(&self) -> PathBuf;

    fn created(&self) -> &DateTime<Utc>;
    fn updated(&self) -> Option<&DateTime<Utc>>;
}

pub trait Container: Common {}
