use super::RedisValue;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct RedisHashMap {
    pub data: HashMap<RedisValue, RedisValue>,
}

impl RedisHashMap {
    pub fn new(data: HashMap<RedisValue, RedisValue>) -> Self {
        Self { data }
    }
}
