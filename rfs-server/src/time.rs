use std::time::{Duration, SystemTime};

#[inline]
pub fn utc_now() -> Option<Duration> {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => Some(d),
        Err(_) => None
    }
}
