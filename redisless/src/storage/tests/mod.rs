use std::{thread::sleep, time::Duration};

use crate::storage::Storage;
use crate::storage::{in_memory::InMemoryStorage, models::Expiry};

#[test]
fn test_in_memory_storage() {
    let mut mem = InMemoryStorage::new();
    mem.write(b"key", b"xxx");
    assert_eq!(mem.read(b"key"), Some(&b"xxx"[..]));
    assert_eq!(mem.remove(b"key"), 1);
    assert_eq!(mem.remove(b"key"), 0);
    assert_eq!(mem.read(b"does not exist"), None);
}

#[test]
fn test_dbsize() {
    let mut mem = InMemoryStorage::new();
    mem.write(b"key", b"xxx");
    assert_eq!(mem.size(), 1);
    assert_eq!(mem.remove(b"key"), 1);
    assert_eq!(mem.size(), 0);
    assert_eq!(mem.remove(b"key"), 0);
    assert_eq!(mem.size(), 0);
    mem.write(b"key", b"xxx");
    mem.write(b"key2", b"xxx");
    assert_eq!(mem.size(), 2);
}

#[test]
fn test_expire() {
    let mut mem = InMemoryStorage::new();

    let duration: u64 = 4;
    mem.write(b"key", b"xxx");
    if let Ok(e) = Expiry::new_from_secs(duration) {
        let ret_val = mem.expire(b"key", e);
        assert_eq!(ret_val, 1);
        assert_eq!(mem.read(b"key"), Some(&b"xxx"[..]));
        sleep(Duration::from_secs(duration));
        assert_eq!(mem.read(b"key"), None);
    }

    let duration: u64 = 1738;
    mem.write(b"key", b"xxx");
    if let Ok(e) = Expiry::new_from_millis(duration) {
        let ret_val = mem.expire(b"key", e);
        assert_eq!(ret_val, 1);
        assert_eq!(mem.read(b"key"), Some(&b"xxx"[..]));
        sleep(Duration::from_millis(duration));
        assert_eq!(mem.read(b"key"), None);
    }
}

#[test]
fn contains() {
    let mut mem = InMemoryStorage::new();
    mem.write(b"key1", b"value1");
    let x = mem.contains(b"key1");
    assert_eq!(x, true);
    let x = mem.contains(b"key2");
    assert_eq!(x, false);
}
