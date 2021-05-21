use chrono::{offset::Utc, Duration};

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Expiry {
    pub timestamp: i64,
}

#[derive(Debug)]
pub struct TimeOverflow {}

impl Expiry {
    pub fn new_from_millis(duration: u64) -> Result<Self, TimeOverflow> {
        Utc::now()
            .checked_add_signed(Duration::milliseconds(duration as i64))
            .map(|t| Self {
                timestamp: t.timestamp_millis(),
            })
            .ok_or(TimeOverflow {})
    }

    pub fn new_from_secs(duration: u64) -> Result<Self, TimeOverflow> {
        Utc::now()
            .checked_add_signed(Duration::seconds(duration as i64))
            .map(|t| Self {
                timestamp: t.timestamp_millis(),
            })
            .ok_or(TimeOverflow {})
    }

    pub fn duration_left_millis(&self) -> i64 {
        self.timestamp - Utc::now().timestamp_millis()
    }
}
