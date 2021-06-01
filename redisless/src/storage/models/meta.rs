use std::sync::atomic::{AtomicBool, Ordering};

use super::{Expiry, RedisType};

pub struct RedisMeta {
    pub data_type: RedisType,
    pub expiry: Option<Expiry>,
    pub tombstone: AtomicBool,
}

impl RedisMeta {
    pub fn new(data_type: RedisType, expiry: Option<Expiry>) -> Self {
        Self {
            data_type,
            expiry,
            tombstone: AtomicBool::from(false),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.tombstone.load(Ordering::SeqCst) || {
            if let Some(expiry) = &self.expiry {
                if expiry.duration_left_millis() <= 0 {
                    self.tombstone.store(true, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
}
