use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::storage::Storage;

#[derive(Debug, PartialEq)]
pub struct Expiry {
    timestamp: Instant,
    pub duration: u64,
}

impl Expiry {
    pub fn new(duration: u64) -> Self {
        Self {
            timestamp: Instant::now(),
            duration,
        }
    }
}

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
        if let Some(expiry) = &self.expiry {
            if expiry.timestamp.elapsed() >= Duration::from_secs(expiry.duration) {
                return true;
            }
        }
        false
    }
}

pub struct InMemoryStorage {
    data_mapper: HashMap<Vec<u8>, DataType>,
    string_store: HashMap<Vec<u8>, RedisValue>,
}

enum DataType {
    String,
    List,
    Set,
    Hash,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {
            data_mapper: HashMap::new(),
            string_store: HashMap::new(),
        }
    }
}

impl Storage for InMemoryStorage {
    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.data_mapper.insert(key.to_vec(), DataType::String);
        self.string_store
            .insert(key.to_vec(), RedisValue::new(value.to_vec(), None));
    }

    fn expire(&mut self, key: &[u8], duration: u64) -> u32 {
        if let Some(value) = self.string_store.get_mut(key) {
            value.expiry = Some(Expiry::new(duration));
            1 // timeout was set
        } else {
            0 // key does not exist
        }
    }

    fn setex(&mut self, key: &[u8], value: &[u8], duration: u64) {
        self.data_mapper.insert(key.to_vec(), DataType::String);
        self.string_store.insert(
            key.to_vec(),
            RedisValue::new(value.to_vec(), Some(Expiry::new(duration))),
        );
    }

    fn get(&mut self, key: &[u8]) -> Option<&[u8]> {
        if let Some(value) = self.string_store.get(key) {
            match value.is_expired() {
                true => {
                    self.del(key);
                    None
                }
                false => Some(&self.string_store.get(key).unwrap().data[..]),
            }
        } else {
            None
        }
    }

    fn del(&mut self, key: &[u8]) -> u32 {
        match self.data_mapper.get(key) {
            Some(data_type) => match data_type {
                DataType::String => match self.string_store.remove(key) {
                    Some(_) => 1,
                    None => 0,
                },
                DataType::List => 0,
                DataType::Set => 0,
                DataType::Hash => 0,
            },
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use crate::storage::in_memory::InMemoryStorage;
    use crate::storage::Storage;

    #[test]
    fn test_in_memory_storage() {
        let mut mem = InMemoryStorage::new();
        mem.set(b"key", b"xxx");
        assert_eq!(mem.get(b"key"), Some(&b"xxx"[..]));
        assert_eq!(mem.del(b"key"), 1);
        assert_eq!(mem.del(b"key"), 0);
        assert_eq!(mem.get(b"does not exist"), None);
    }

    #[test]
    fn test_setex() {
        let mut mem = InMemoryStorage::new();
        let duration: u64 = 4;
        mem.setex(b"key", b"xxx", 4);
        assert_eq!(mem.get(b"key"), Some(&b"xxx"[..]));
        sleep(Duration::from_secs(duration));
        assert_eq!(mem.get(b"xxx"), None);
    }

    #[test]
    fn test_expire() {
        let mut mem = InMemoryStorage::new();
        let duration: u64 = 4;
        mem.set(b"key", b"xxx");
        mem.expire(b"key", duration);
        assert_eq!(mem.get(b"key"), Some(&b"xxx"[..]));
        sleep(Duration::from_secs(duration));
        assert_eq!(mem.get(b"key"), None);
    }
}
