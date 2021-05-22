#[cfg(test)]
mod tests;

pub mod in_memory;
pub mod models;

use std::collections::HashMap;

use models::expiry::Expiry;
use models::RedisString;

use self::models::RedisMeta;

pub trait Storage {
    fn write(&mut self, key: &[u8], value: &[u8]);
    fn expire(&mut self, key: &[u8], expiry: Expiry) -> u32;
    fn read(&mut self, key: &[u8]) -> Option<&[u8]>;
    fn remove(&mut self, key: &[u8]) -> u32;
    fn contains(&mut self, key: &[u8]) -> bool;
    fn hwrite(&mut self, key: &[u8], value: HashMap<RedisString, RedisString>);
    fn hread(&mut self, key: &[u8], field_key: &[u8]) -> Option<&[u8]>;
    fn size(&self) -> u64;
    fn meta(&self, key: &[u8]) -> Option<&RedisMeta>;
}
