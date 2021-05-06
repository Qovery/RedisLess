use std::{collections::HashMap, usize};

use crate::Storage;

type Expiry = Option<usize>;

#[derive(Debug, PartialEq)]
pub struct RedisValue {
    pub data: Vec<u8>,
    expiry: Expiry,
}

impl RedisValue {
    pub fn new(data: Vec<u8>, expiry: Expiry) -> Self {
        RedisValue { data, expiry }
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
        self.string_store.insert(key.to_vec(), RedisValue::new(value.to_vec(), None));
    }

    fn expire(&mut self, key: &[u8], expiry: usize) {
        self.string_store.entry(key.to_vec()).and_modify(|v| v.expiry = Some(expiry));
    }

    fn setex(&mut self, key: &[u8], value: &[u8], expiry: usize) {
        self.data_mapper.insert(key.to_vec(), DataType::String);
        self.string_store.insert(key.to_vec(), RedisValue::new(value.to_vec(), Some(expiry)));
    }

    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.string_store.get(key).map(|v| &v.data[..])
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
}
