use redis::{Commands, Connection, RedisResult};
use std::{thread::sleep, time::Duration};

use crate::command::command_error::RedisCommandError;
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

    let response: Result<i64, redis::RedisError> = con.incr("63", "foo");
    match response {
        Ok(_) => panic!("got valid response from incr command for key {} and value {}", "63", "foo"),
        Err(error) => {
            assert_eq!(error.kind(), redis::ErrorKind::ExtensionError);
            assert_eq!(error.to_string(), "invalid: digit found in string");
        }
    };

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
fn llen() {
    let (server, mut con) = get_redis_client_connection(3377);

    let values = &["val1", "val2", "val3", "val4"];
    let x = con
        .rpush::<&'static str, &[&str], u32>("lkey", values)
        .unwrap();

    let l: i64 = con.llen("lkey").unwrap();
    assert_eq!(l, 4);

    let x: i64 = con.llen("new_key").unwrap();
    assert_eq!(x, 0);

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
fn rpushx_lpushx() {
    let (server, mut con) = get_redis_client_connection(3352);
    let values = &["val1", "val2", "val3"][..];
    let a = con
        .rpush_exists::<&'static str, &[&str], u32>("new_key", values)
        .unwrap();
    assert_eq!(a, 0);
    let b = con
        .lpush_exists::<&'static str, &[&str], u32>("new_key", values)
        .unwrap();
    assert_eq!(b, 0);

    let _ = con
        .rpush::<&'static str, &[&str], u32>("listkey", values)
        .unwrap();
    let values2 = &["val4", "val5", "val6"][..];

    let x = con
        .rpush_exists::<&'static str, &[&str], u32>("listkey", values2)
        .unwrap();
    assert_eq!(x, 6);
    let y = con
        .lpush_exists::<&'static str, &[&str], u32>("listkey", values2)
        .unwrap();
    assert_eq!(y, 9);

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn rpop_lpop() {
    let (server, mut con) = get_redis_client_connection(3354);
    let values = &["val1", "val2"][..];
    let _ = con
        .rpush::<&'static str, &[&str], u32>("listkey", values)
        .unwrap();

    let x: String = con.rpop("listkey").unwrap();
    assert_eq!(x, "val2");
    let y: String = con.lpop("listkey").unwrap();
    assert_eq!(y, "val1");
    let z: bool = con.exists("listkey").unwrap();
    assert_eq!(z, false);
    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn lindex_lset_linsert() {
    let (server, mut con) = get_redis_client_connection(3355);
    let values = &["val1", "val2", "val3"][..];
    let _ = con
        .rpush::<&'static str, &[&str], u32>("listkey", values)
        .unwrap();

    let x: String = con.lindex("listkey", 1).unwrap();
    assert_eq!(x, "val2");
    let y: String = con.lindex("listkey", -3).unwrap();
    assert_eq!(y, "val1");

    let _: () = con.lset("listkey", 0, "val0").unwrap();

    let a: i64 = con.linsert_before("listkey", "val2", "val1").unwrap();
    assert_eq!(a, 4);
    let b: i64 = con.linsert_after("listkey", "val3", "val4").unwrap();
    assert_eq!(b, 5);
    let c: i64 = con.linsert_before("listkey", "val10", "val9").unwrap();
    assert_eq!(c, -1);
    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn ltrim_lrem_rpoplpush() {
    let (server, mut con) = get_redis_client_connection(3356);
    let values = &["val1", "val2", "val3", "val4", "val5", "val6"][..];
    let _ = con
        .rpush::<&'static str, &[&str], u32>("listkey", values)
        .unwrap();
    let _: () = con.ltrim("listkey", 1, 3).unwrap();

    let nums = &[1, 2, 1, 1, 1, 2, 1, 1][..];
    let _ = con
        .rpush::<&'static str, &[u64], u32>("numkey", nums)
        .unwrap();
    let x: i64 = con.lrem("numkey", -3, 1).unwrap();
    assert_eq!(x, 3);
    let y: i64 = con.lrem("numkey", 2, 2).unwrap();
    assert_eq!(y, 2);

    let src_vals = &["val1", "val2"];
    let dest_vals = &["val3", "val4"];
    let _ = con
        .rpush::<&'static str, &[&str], u32>("src", src_vals)
        .unwrap();
    let _ = con
        .rpush::<&'static str, &[&str], u32>("dest", dest_vals)
        .unwrap();
    let a: String = con.rpoplpush("src", "dest").unwrap();
    assert_eq!(a, "val2");
    let b: String = con.rpoplpush("src", "dest").unwrap();
    assert_eq!(b, "val1");
    let c: bool = con.exists("src").unwrap();
    assert_eq!(c, false);

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

#[test]
#[serial]
fn sadd_scard_srem() {
    let (server, mut con) = get_redis_client_connection(3357);

    let values = &["val1", "val2", "val3", "val1"][..];
    let x: i64 = con.sadd("setkey", values).unwrap();
    assert_eq!(x, 3);
    let values2 = &["val3", "val4", "val5"][..];
    let y: i64 = con.sadd("setkey", values2).unwrap();
    assert_eq!(y, 2);
    let a: i64 = con.scard("nokey").unwrap();
    assert_eq!(a, 0);
    let b: i64 = con.scard("setkey").unwrap();
    assert_eq!(b, 5);
    let values3 = &["val3", "val5", "val7"][..];
    let i: i64 = con.srem("setkey", values3).unwrap();
    assert_eq!(i, 2);
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
