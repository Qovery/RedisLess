use super::RedisValue;

#[derive(Debug, PartialEq)]
pub struct RedisString {
    pub data: RedisValue,
}

impl RedisString {
    pub fn new(data: RedisValue) -> Self {
        Self { data }
    }
}
