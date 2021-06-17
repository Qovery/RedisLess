use redis::{Commands, Connection, RedisResult};
use std::{thread::sleep, time::Duration};

use crate::server::ServerState;
use crate::storage::in_memory::InMemoryStorage;
use crate::Server;

fn get_redis_client_connection(port: u16) -> (Server, Connection) {
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));

    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    (server, redis_client.get_connection().unwrap())
}
#[test]
#[serial]
fn test_incr_decr_commands() {
    let (server, mut con) = get_redis_client_connection(3365);

    let _: () = con.set("some_number", "12").unwrap();
    let _: () = con.incr("some_number", 1).unwrap();
    let some_number: u32 = con.get("some_number").unwrap();
    assert_eq!(some_number, 13_u32);

    let _: () = con.set("n", "100").unwrap();
    let _: () = con.decr("n", 1).unwrap();
    let n: u32 = con.get("n").unwrap();
    assert_eq!(n, 99_u32);

    let _: () = con.set("0", "12").unwrap();
    let _: () = con.incr("0", 500).unwrap();
    let value: u32 = con.get("0").unwrap();
    assert_eq!(value, 512_u32);

    let _: () = con.set("63", "89").unwrap();
    let _: () = con.decr("63", 10).unwrap();
    let value: u32 = con.get("63").unwrap();
    assert_eq!(value, 79_u32);

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn test_redis_implementation() {
    let (server, mut con) = get_redis_client_connection(3366);

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
    let _: () = con.incr("intkey", 1).unwrap();

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
fn expire_and_ttl() {
    let (server, mut con) = get_redis_client_connection(3369);

    let ttl: i32 = con.ttl("key").unwrap();
    assert_eq!(ttl, -2);
    let ttl: i32 = con.pttl("key").unwrap();
    assert_eq!(ttl, -2);
    // EXPIRE
    let duration: usize = 50;
    let _: () = con.set("key", "value").unwrap();
    let x: String = con.get("key").unwrap();
    assert_eq!(x, "value");
    let ttl: i32 = con.ttl("key").unwrap();
    assert_eq!(ttl, -1);

    let ret_val: u32 = con.pexpire("key", duration).unwrap();
    assert_eq!(ret_val, 1);
    let ttl: i32 = con.pttl("key").unwrap();
    assert!(ttl <= duration as i32 && duration as i32 - 30 < ttl);
    let ttl: i32 = con.ttl("key").unwrap();
    assert_eq!(ttl, (duration / 1000) as i32);
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
    let ttl: i32 = con.pttl("key").unwrap();
    assert!(ttl <= duration as i32 && duration as i32 - 30 < ttl);
    sleep(Duration::from_millis(duration as u64));
    let x: Option<String> = con.get("key").ok();
    assert_eq!(x, None);
    let ttl: i32 = con.pttl("key").unwrap();
    assert_eq!(ttl, -2);

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
    let (server, mut con) = get_redis_client_connection(3332);

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
fn dbsize() {
    let (server, mut con) = get_redis_client_connection(3331);

    let x: u64 = redis::cmd("DBSIZE").query(&mut con).unwrap(); //con.dbsize().unwrap();
    assert_eq!(x, 0);
    let _: String = con.set("key1", "valueA").unwrap();
    let x: u64 = redis::cmd("DBSIZE").query(&mut con).unwrap(); //con.dbsize().unwrap();
    assert_eq!(x, 1);
    let _: String = con.set("key2", "valueA").unwrap();
    let x: u64 = redis::cmd("DBSIZE").query(&mut con).unwrap(); //con.dbsize().unwrap();
    assert_eq!(x, 2);
    let _: u32 = con.del("key2").unwrap();
    let x: u64 = redis::cmd("DBSIZE").query(&mut con).unwrap(); //con.dbsize().unwrap();
    assert_eq!(x, 1);
    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn mset() {
    let (server, mut con) = get_redis_client_connection(3343);

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
    let (server, mut con) = get_redis_client_connection(3342);

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
    let (server, mut con) = get_redis_client_connection(3345);

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
    let (server, mut con) = get_redis_client_connection(3347);

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

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn rpush() {
    let (server, mut con) = get_redis_client_connection(3344);
    let values = &["val1", "val2", "val3"][..];
    let x = con
        .rpush::<&'static str, &[&str], u32>("listkey", values)
        .unwrap();
    assert_eq!(x, 3);

    let values2 = &["val4", "val5", "val6"][..];
    let y = con
        .rpush::<&'static str, &[&str], u32>("listkey", values2)
        .unwrap();
    assert_eq!(y, 6);
    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn lpush() {
    let (server, mut con) = get_redis_client_connection(3348);
    let values = &["val1", "val2", "val3"][..];
    let _ = con
        .rpush::<&'static str, &[&str], u32>("listkey", values)
        .unwrap();

    let values2 = &["val4", "val5", "val6"][..];
    let x = con
        .lpush::<&'static str, &[&str], u32>("listkey", values2)
        .unwrap();
    assert_eq!(x, 6);
    let y = con
        .lpush::<&'static str, &[&str], u32>("listkey2", values2)
        .unwrap();
    assert_eq!(y, 3);
    assert_eq!(server.stop(), Some(ServerState::Stopped));
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

#[test]
fn append() {
    let (server, mut con) = get_redis_client_connection(3346);

    let _: String = con.set("key1", "value1").unwrap();
    let len: usize = con.append("key1", "value1").unwrap();
    assert_eq!(len, 12);
    let x: String = con.get("key1").unwrap();
    assert_eq!(x, "value1value1");

    let len: usize = con.append("key2", "value2").unwrap();
    assert_eq!(len, 6);
    let x: String = con.get("key2").unwrap();
    assert_eq!(x, "value2");

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}
