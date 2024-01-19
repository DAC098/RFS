pub mod manager;

pub type Version = u64;
pub type Timestamp = u64;

pub const PASSWORDS_KEY_INFO: &[u8; 9] = b"passwords";
pub const SESSIONS_KEY_INFO: &[u8; 8] = b"sessions";

