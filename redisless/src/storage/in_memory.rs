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
                false => Some(self.string_store.get(key).unwrap()),
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

    /// If the key was present **and** the key was not expired, return `true`
    ///
    /// If the key present but was expired, remove the key and return `false`
    ///
    /// If the key was not present at all, return `false`
    fn contains(&mut self, key: &[u8]) -> bool {
        if let Some(meta) = self.data_mapper.get(key) {
            match meta.is_expired() {
                true => {
                    self.remove(key);
                    false
                }
                false => true,
            }
        } else {
            false
        }
    }

    fn hwrite(&mut self, key: &[u8], value: HashMap<RedisString, RedisString>) {
        let meta = RedisMeta::new(RedisType::Hash, None);
        self.data_mapper.insert(key.to_vec(), meta);
        self.hash_store
            .insert(key.to_vec(), RedisHashMap::new(value));
    }

    fn hread(&mut self, key: &[u8], field_key: &[u8]) -> Option<&[u8]> {
        if let Some(meta) = self.data_mapper.get(key) {
            match meta.is_expired() {
                true => {
                    self.remove(key);
                    None
                }
                // good to go
                false => {
                    // will never panic since we already checked if the key existed in data_mapper
                    if let Some(field_value) = self.hash_store.get(key).unwrap().data.get(field_key) {
                        Some(field_value)
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        }
    }
}
