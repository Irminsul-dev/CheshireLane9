use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_timestamp_s() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn get_month_timestamp_s() -> i64 {
    // ponytail: good enough for old monthly-shop placeholder; replace with calendar month start if rewards depend on real reset boundaries.
    now_timestamp_s()
}
