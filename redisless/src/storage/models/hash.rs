use super::RedisString;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct RedisHashMap {
    pub data: HashMap<RedisString, RedisString>,
}

impl RedisHashMap {
    pub fn new(data: HashMap<RedisString, RedisString>) -> Self {
        Self { data }
    }
}
