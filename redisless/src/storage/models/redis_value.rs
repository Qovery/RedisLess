use super::Expiry;
use std::time::Instant;

#[derive(Debug, PartialEq)]
pub struct RedisValue {
    pub data: Vec<u8>,
    pub expiry: Option<Expiry>,
}

impl RedisValue {
    pub fn new(data: Vec<u8>, expiry: Option<Expiry>) -> Self {
        RedisValue { data, expiry }
    }

    pub fn is_expired(&self) -> bool {
        match &self.expiry {
            Some(expiry) if expiry.timestamp <= Instant::now() => true,
            _ => false,
        }
    }
}
