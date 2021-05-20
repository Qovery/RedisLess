use redis::{cmd, Commands, RedisResult};
use std::{thread::sleep, time::Duration};

use crate::server::ServerState;
use crate::storage::in_memory::InMemoryStorage;
use crate::Server;

#[test]
#[serial]
fn test_redis_implementation() {
    let port = 3366;
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));
    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    let mut con = redis_client.get_connection().unwrap();

    let _: () = con.set("key", "value").unwrap();
    let exists: bool = con.exists("key").unwrap();
    assert_eq!(exists, true);
    let x: String = con.get("key").unwrap();
    assert_eq!(x, "value");

    let x: RedisResult<String> = con.get("not-existing-key");
    assert_eq!(x.is_err(), true);
    let exists: bool = con.exists("non-existant-key").unwrap();
    assert_eq!(exists, false);

    let x: u32 = con.del("key").unwrap();
    assert_eq!(x, 1);

    let x: u32 = con.del("key").unwrap();
    assert_eq!(x, 0);

    let _: () = con.set("key2", "original value").unwrap();
    let x: String = con.get("key2").unwrap();
    assert_eq!(x, "original value");

    let x: u32 = con.set_nx("key2", "new value").unwrap();
    assert_eq!(x, 0);

    let x: String = con.get("key2").unwrap();
    assert_eq!(x, "original value");

    let x: u32 = con.set_nx("key3", "value3").unwrap();
    assert_eq!(x, 1);

    let x: String = con.get("key3").unwrap();
    assert_eq!(x, "value3");

    let _: () = con.set("intkey", "10").unwrap();
    let _ = con
        .send_packed_command(cmd("INCR").arg("intkey").get_packed_command().as_slice())
        .unwrap();

    let x: u32 = con.get("intkey").unwrap();
    assert_eq!(x, 11u32);

    let _: () = con.set("intkeyby", "10").unwrap();
    let _: () = con.incr("intkeyby", "10").unwrap();

    let x: u32 = con.get("intkeyby").unwrap();
    assert_eq!(x, 20u32);

    let _: () = con.set("intkeybyneg", "10").unwrap();
    let _: () = con.incr("intkeybyneg", "-5").unwrap();

    let x: u32 = con.get("intkeybyneg").unwrap();
    assert_eq!(x, 5u32);

    let _: () = con.incr("keydoesnotexist", "20").unwrap();

    let x: u32 = con.get("keydoesnotexist").unwrap();
    assert_eq!(x, 20u32);

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn expire() {
    let port = 3359;
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));
    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    let mut con = redis_client.get_connection().unwrap();

    // EXPIRE
    let duration: usize = 2;
    let _: () = con.set("key", "value").unwrap();
    let x: String = con.get("key").unwrap();
    assert_eq!(x, "value");

    let ret_val: u32 = con.pexpire("key", duration).unwrap();
    assert_eq!(ret_val, 1);
    sleep(Duration::from_millis(duration as u64));
    let x: Option<String> = con.get("key").ok();
    assert_eq!(x, None);

    // PEXPIRE
    let duration: usize = 2387;
    let _: () = con.set("key", "value").unwrap();
    let x: String = con.get("key").unwrap();
    assert_eq!(x, "value");

    let ret_val: u32 = con.pexpire("key", duration).unwrap();
    assert_eq!(ret_val, 1);
    sleep(Duration::from_millis(duration as u64));
    let x: Option<String> = con.get("key").ok();
    assert_eq!(x, None);

    // SETEX
    let duration: usize = 2;
    let _: () = con.set_ex("key", "value", duration).unwrap();
    let x: String = con.get("key").unwrap();
    assert_eq!(x, "value");

    sleep(Duration::from_secs(duration as u64));
    let x: Option<String> = con.get("key").ok();
    assert_eq!(x, None);

    // PSETEX
    let duration = 1984_usize;
    let _: () = con.pset_ex("key", "value", duration).unwrap();
    let x: String = con.get("key").unwrap();
    assert_eq!(x, "value");
    sleep(Duration::from_millis(duration as u64));
    let x: Option<String> = con.get("key").ok();
    assert_eq!(x, None);

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn get_set() {
    let port = 3332;
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));

    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    let mut con = redis_client.get_connection().unwrap();

    let _: String = con.set("key1", "valueA").unwrap();
    let x: String = con.getset("key1", "valueB").unwrap();
    assert_eq!(x, "valueA");

    let x: String = con.getset("key1", "valueC").unwrap();
    assert_eq!(x, "valueB");

    let x: String = con.get("key1").unwrap();
    assert_eq!(x, "valueC");

    let x: Option<String> = con.getset("key2", "value2").ok();
    assert_eq!(x, None);

    let x: String = con.get("key2").unwrap();
    assert_eq!(x, "value2");

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn mset() {
    // make these first 5 lines into a macro?
    let port = 3343;
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));
    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    let mut con = redis_client.get_connection().unwrap();

    let key_value_pairs = &[("key0", "val0"), ("key1", "val1"), ("key2", "val2")][..];

    let _ = con.set_multiple::<&'static str, &'static str, u32>(key_value_pairs);

    let exes: Vec<String> = con.get(&["key0", "key1", "key2"][..]).unwrap();
    assert_eq!(exes[0], "val0");
    assert_eq!(exes[1], "val1");
    assert_eq!(exes[2], "val2");

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn mset_nx() {
    let port = 3342;
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));
    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    let mut con = redis_client.get_connection().unwrap();

    let key_value_pairs = &[("key0", "val0"), ("key1", "val1"), ("key2", "val2")][..];

    let x = con
        .mset_nx::<&'static str, &'static str, u32>(key_value_pairs)
        .unwrap();
    assert_eq!(x, 1); // keys were set

    let exes: Vec<String> = con.get(&["key0", "key1", "key2"][..]).unwrap();
    assert_eq!(exes[0], "val0");
    assert_eq!(exes[1], "val1");
    assert_eq!(exes[2], "val2");

    let key_value_pairs = &[("key4", "val4"), ("key1", "val5")][..];
    let x = con
        .mset_nx::<&'static str, &'static str, u32>(key_value_pairs)
        .unwrap();
    assert_eq!(x, 0); // a key was repeated so none were set
    let x: String = con.get("key1").unwrap();
    assert_eq!(x, "val1"); // key1 is still val1
    let x: Option<String> = con.get("key4").ok();
    assert_eq!(x, None); // key4 was not set

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn mget() {
    let port = 3346;
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));
    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    let mut con = redis_client.get_connection().unwrap();

    let key_value_pairs = &[("key0", "val0"), ("key1", "val1"), ("key2", "val2")][..];

    let _ = con.set_multiple::<&'static str, &'static str, u32>(key_value_pairs);

    let keys = vec!["key0", "key1", "key2"];
    let exes: Vec<String> = con.get(keys).unwrap();
    assert_eq!(exes[0], "val0");
    assert_eq!(exes[1], "val1");
    assert_eq!(exes[2], "val2");

    let _: () = con.del("key0").unwrap();
    let _: () = con.del("key1").unwrap();

    let keys = vec!["key0", "key1", "key2"];
    let exes: Vec<Option<String>> = con.get(keys).unwrap();
    assert_eq!(exes[0], None);
    assert_eq!(exes[1], None);
    assert_eq!(exes[2], Some("val2".to_string()));

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn hset() {
    let port = 3347;
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));
    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    let mut con = redis_client.get_connection().unwrap();

    let key_value_pairs = &[("fkey0", "val0"), ("fkey1", "val1"), ("fkey2", "val2")][..];
    let _: () = con
        .hset_multiple::<&'static str, &'static str, &'static str, ()>("key0", key_value_pairs)
        .unwrap();
    let x: String = con.hget("key0", "fkey0").unwrap();
    assert_eq!(x, "val0");
    let x: String = con.hget("key0", "fkey1").unwrap();
    assert_eq!(x, "val1");
    let x: String = con.hget("key0", "fkey2").unwrap();
    assert_eq!(x, "val2");
    let x: Option<String> = con.hget("key0", "fkey3").ok();
    assert_eq!(x, None);
    let x: Option<String> = con.hget("key1", "fkey3").ok();
    assert_eq!(x, None);
}

#[test]
#[serial]
fn start_and_stop_server() {
    let server = Server::new(InMemoryStorage::new(), 3340);
    assert_eq!(server.start(), Some(ServerState::Started));
    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
fn start_and_stop_server_multiple_times() {
    let server = Server::new(InMemoryStorage::new(), 3341);
    for _ in 0..9 {
        assert_eq!(server.start(), Some(ServerState::Started));
        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }
}
