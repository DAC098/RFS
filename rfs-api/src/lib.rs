use std::collections::HashMap;

mod response;

pub mod error;
pub use error::{ApiError, ApiErrorKind, Detail};

pub mod traits;
pub use traits::Validator;

pub mod users;
pub mod auth;
pub mod sec;
pub mod fs;

mod payload;
pub use payload::Payload;

pub type Tags = HashMap<String, Option<String>>;

#[cfg(feature = "client")]
pub mod client;
