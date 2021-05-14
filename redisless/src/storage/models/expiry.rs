use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Expiry {
    pub timestamp: Instant,
}

#[derive(Debug)]
pub struct TimeOverflow {}

impl Expiry {
    pub fn new_from_millis(duration: u64) -> Result<Self, TimeOverflow> {
        Instant::now()
            .checked_add(Duration::from_millis(duration))
            .map(|t| Self { timestamp: t })
            .ok_or(TimeOverflow {})
    }

    pub fn new_from_secs(duration: u64) -> Result<Self, TimeOverflow> {
        Instant::now()
            .checked_add(Duration::from_secs(duration))
            .map(|t| Self { timestamp: t })
            .ok_or(TimeOverflow {})
    }
}
