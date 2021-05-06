use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Storage;

pub struct InMemoryStorage {
    data_mapper: HashMap<Vec<u8>, DataType>,
    expiration_store: HashMap<Vec<u8>, Vec<u8>>,
    string_store: HashMap<Vec<u8>, Vec<u8>>,
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
            expiration_store: HashMap::new(),
            string_store: HashMap::new(),
        }
    }
}

impl Storage for InMemoryStorage {
    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.data_mapper.insert(key.to_vec(), DataType::String);
        self.string_store.insert(key.to_vec(), value.to_vec());
    }

    fn get(&mut self, key: &[u8]) -> Option<&[u8]> {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(now) => {
                match self.expiration_store.get(key).map(|v| &v[..]) {
                    None => self.string_store.get(key).map(|v| &v[..]),
                    Some(expiration) => {
                        if std::str::from_utf8(expiration).unwrap().parse::<u64>().unwrap() > now.as_secs() {
                            self.string_store.get(key).map(|v| &v[..])
                        } else {
                            self.expiration_store.remove(key);
                            self.string_store.remove(key);
                            None
                        }
                    }
                }
            }
            Err(_) => panic!("SystemTime before UNIX EPOCH!?")
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

    fn expire(&mut self, key: &[u8], value: &[u8]) -> u32 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(now) => {
                if let Ok(expiration_value) = std::str::from_utf8(value).unwrap().parse::<u64>() {
                    let expiration = expiration_value.checked_add(now.as_secs()).unwrap();
                    self.expiration_store.insert(key.to_vec(), expiration.to_string().as_bytes().to_vec());
                    1
                } else {
                    panic!("Not u64?!")
                }
            }
            Err(_) => panic!("SystemTime before UNIX EPOCH!?")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

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
    fn test_expiration() {
        let mut mem = InMemoryStorage::new();
        mem.set(b"toexpire", b"yyy");
        assert_eq!(mem.get(b"toexpire"), Some(&b"yyy"[..]));
        assert_eq!(mem.expire(b"toexpire", b"1"), 1);

        // before expiring
        assert_eq!(mem.get(b"toexpire"), Some(&b"yyy"[..]));

        // sleep over 1s
        thread::sleep(Duration::from_millis(1001));

        // after expiring
        assert_eq!(mem.get(b"toexpire"), None);
    }
}
