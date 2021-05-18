use std::time::Instant;

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
        match &self.expiry {
            Some(expiry) if expiry.timestamp <= Instant::now() => true,
            _ => false,
        }
    }
}