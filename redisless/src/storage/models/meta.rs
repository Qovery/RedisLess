use super::{Expiry, RedisType};

pub struct RedisMeta {
    pub data_type: RedisType,
    pub expiry: Option<Expiry>,
}

impl RedisMeta {
    pub fn new(data_type: RedisType, expiry: Option<Expiry>) -> Self {
        Self { data_type, expiry }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = &self.expiry {
            expiry.duration_left_millis() <= 0
        } else {
            false
        }
    }
}
