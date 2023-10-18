use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};

#[inline]
pub fn utc_now() -> Option<Duration> {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => Some(d),
        Err(_) => None
    }
}

pub fn utc_secs_to_chrono_datetime(secs: u64) -> Option<DateTime<Utc>> {
    let Ok(secs): Result<i64, _> = TryFrom::try_from(secs) else {
        return None;
    };

    DateTime::from_timestamp(secs, 0)
}

pub fn utc_to_chrono_datetime(duration: &Duration) -> Option<DateTime<Utc>> {
    let Ok(secs): Result<i64, _> = TryFrom::try_from(duration.as_secs()) else {
        return None;
    };

    DateTime::from_timestamp(secs, duration.subsec_nanos())
}
