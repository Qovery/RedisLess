use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::Storage;
use std::ops::Sub;
use std::convert::TryInto;

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

    pub fn to_expire(&self) -> Option<u64> {
        return if let Some(expiry) = &self.expiry {
            if let Some(to_expire) = expiry.duration.checked_sub(expiry.timestamp.elapsed().as_secs()) {
                Some(to_expire)
            } else {
                Some(0)
            }
        } else {
            None
        };
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

    fn ttl(&mut self, key: &[u8]) -> i64 {
        if let Some(value) = self.string_store.get(key) {
            if let Some(expiry) = value.to_expire() {
                if expiry == 0 {
                    // key shouldn't exist as it's expired
                    self.del(key);
                    return -2
                }
                // key not expired yet
                expiry.try_into().unwrap()
            } else {
                // key without expiration
                -1
            }
        } else {
            // key doesn't exist
            -2
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

    use crate::in_memory::InMemoryStorage;
    use crate::Storage;

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

    #[test]
    fn test_ttl() {
        let mut mem = InMemoryStorage::new();
        let duration: u64 = 4;
        mem.set(b"key", b"xxx");
        mem.expire(b"key", duration);

        let ttl1 = mem.ttl(b"key");
        assert_eq!(4, ttl1);

        // sleep 1s, return ttl equal 4-1 -> 3
        sleep(Duration::from_secs(1));
        let ttl2 = mem.ttl(b"key");
        assert_eq!(3, ttl2);

        sleep(Duration::from_secs(4));
        let ttl3 = mem.ttl(b"key");
        // key no longer exists
        assert_eq!(-2, ttl3);

        // key exists, but has no expiration
        mem.set(b"key_no_ttl", b"xxx");
        let ttl4 = mem.ttl(b"key_no_ttl");
        assert_eq!(-1, ttl4);
    }
}
