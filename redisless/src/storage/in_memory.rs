use std::collections::HashMap;

use super::models::{DataType, Expiry, RedisValue};
use crate::storage::Storage;

pub struct InMemoryStorage {
    data_mapper: HashMap<Vec<u8>, DataType>,
    string_store: HashMap<Vec<u8>, RedisValue>,
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
    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.data_mapper.insert(key.to_vec(), DataType::String);
        self.string_store
            .insert(key.to_vec(), RedisValue::new(value.to_vec(), None));
    }

    fn expire(&mut self, key: &[u8], expiry: Expiry) -> u32 {
        if let Some(value) = self.string_store.get_mut(key) {
            value.expiry = Some(expiry);
            1 // timeout was set
        } else {
            0 // key does not exist
        }
    }

    fn read(&mut self, key: &[u8]) -> Option<&[u8]> {
        if let Some(value) = self.string_store.get(key) {
            match value.is_expired() {
                true => {
                    self.remove(key);
                    None
                }
                false => Some(&self.string_store.get(key).unwrap().data[..]),
            }
        } else {
            None
        }
    }

    fn remove(&mut self, key: &[u8]) -> u32 {
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

    fn contains(&mut self, key: &[u8]) -> bool {
        self.data_mapper.contains_key(key)
    }
}
