use std::collections::HashMap;

use super::models::*;
use crate::storage::Storage;

pub struct InMemoryStorage {
    data_mapper: HashMap<RedisString, RedisMeta>,
    string_store: HashMap<RedisString, RedisString>,
    hash_store: HashMap<RedisString, RedisHashMap>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            data_mapper: HashMap::new(),
            string_store: HashMap::new(),
            hash_store: HashMap::new(),
        }
    }
}

impl Storage for InMemoryStorage {
    fn write(&mut self, key: &[u8], value: &[u8]) {
        let meta = RedisMeta::new(RedisType::String, None);
        self.data_mapper.insert(key.to_vec(), meta);
        self.string_store.insert(key.to_vec(), value.to_vec());
    }

    fn expire(&mut self, key: &[u8], expiry: Expiry) -> u32 {
        if let Some(meta) = self.data_mapper.get_mut(key) {
            meta.expiry = Some(expiry);
            1 // timeout was set
        } else {
            0 // key does not exist
        }
    }

    fn read(&mut self, key: &[u8]) -> Option<&[u8]> {
        if let Some(value) = self.data_mapper.get(key) {
            match value.is_expired() {
                true => {
                    self.remove(key);
                    None
                }
                false => Some(&self.string_store.get(key).unwrap()[..]),
            }
        } else {
            None
        }
    }

    fn remove(&mut self, key: &[u8]) -> u32 {
        use RedisType::*;
        match self.data_mapper.remove_entry(key) {
            Some((key, meta)) => match meta.data_type {
                String => match self.string_store.remove(&key) {
                    Some(_) => 1,
                    None => 0,
                },
                Hash => match self.hash_store.remove(&key) {
                    Some(_) => 1,
                    None => 0,
                },
                List => unimplemented!(),
                Set => unimplemented!(),
            },
            None => 0,
        }
    }

    fn contains(&mut self, key: &[u8]) -> bool {
        self.data_mapper.contains_key(key)
    }
}
