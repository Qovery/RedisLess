use redis::{Commands, Connection, ToRedisArgs};
use std::{thread::sleep, time::Duration};

use crate::server::ServerState;
use crate::storage::in_memory::InMemoryStorage;
use crate::Server;

struct TestClient {
    server: Server,
    con: Connection,
}

impl Drop for TestClient {
    fn drop(&mut self) {
        assert_eq!(self.server.stop(), Some(ServerState::Stopped));
    }
}

impl TestClient {
    fn connect(port: u16) -> Self {
        let server = Server::new(InMemoryStorage::new(), port);
        assert_eq!(server.start(), Some(ServerState::Started));

        let con = redis::Client::open(format!("redis://127.0.0.1:{}/", port))
            .unwrap()
            .get_connection()
            .unwrap();

        TestClient { server, con }
    }

    fn get<K: ToRedisArgs>(&mut self, key: K) -> Option<String> {
        self.con.get(key).ok()
    }
    fn set<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V) {
        self.con.set(key, value).unwrap()
    }
    fn incr<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, delta: V) {
        self.con.incr(key, delta).unwrap()
    }
    fn decr<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, delta: V) {
        self.con.decr(key, delta).unwrap()
    }
    fn hget<K: ToRedisArgs, F: ToRedisArgs>(&mut self, key: K, field: F) -> Option<String> {
        self.con.hget(key, field).ok()
    }
    fn hset_multiple<K: ToRedisArgs, F: ToRedisArgs, V: ToRedisArgs>(
        &mut self,
        key: K,
        items: &'static [(F, V)],
    ) {
        self.con.hset_multiple(key, items).unwrap()
    }
    fn del<K: ToRedisArgs>(&mut self, key: K) {
        self.con.del(key).unwrap()
    }
    fn exists<K: ToRedisArgs>(&mut self, key: K) -> bool {
        self.con.exists(key).unwrap()
    }
    fn expire<K: ToRedisArgs>(&mut self, key: K, seconds: usize) -> u8 {
        self.con.expire(key, seconds).unwrap()
    }
    fn pexpire<K: ToRedisArgs>(&mut self, key: K, ms: usize) -> u8 {
        self.con.pexpire(key, ms).unwrap()
    }
    fn ttl<K: ToRedisArgs>(&mut self, key: K) -> i32 {
        self.con.ttl(key).unwrap()
    }
    fn pttl<K: ToRedisArgs>(&mut self, key: K) -> i32 {
        self.con.pttl(key).unwrap()
    }
    fn set_multiple<K: ToRedisArgs, V: ToRedisArgs>(&mut self, items: &'static [(K, V)]) {
        self.con.set_multiple(items).unwrap()
    }
    fn set_ex<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V, seconds: usize) {
        self.con.set_ex(key, value, seconds).unwrap()
    }
    fn pset_ex<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V, milliseconds: usize) {
        self.con.pset_ex(key, value, milliseconds).unwrap()
    }
    fn mset_nx<K: ToRedisArgs, V: ToRedisArgs>(&mut self, items: &'static [(K, V)]) -> u8 {
        self.con.mset_nx(items).unwrap()
    }
    fn getset<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V) -> Option<String> {
        self.con.getset(key, value).ok()
    }
    fn append<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V) -> usize {
        self.con.append(key, value).unwrap()
    }
    fn llen<K: ToRedisArgs>(&mut self, key: K) -> usize {
        self.con.llen(key).unwrap()
    }
    fn rpush<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V) -> usize {
        self.con.rpush(key, value).unwrap()
    }
}

#[test]
fn test_client_incr_by_integer() {
    let mut t = TestClient::connect(1024);

    // Arrange
    t.set("integer +", 2500);
    t.set("integer -", "17");
    // Act
    t.incr("integer +", 600);
    t.incr("integer -", -30);
    // Assert
    assert_eq!(t.get("integer +").unwrap(), "3100");
    assert_eq!(t.get("integer -").unwrap(), "-13");
}

#[test]
fn test_client_decr_by_integer() {
    let mut t = TestClient::connect(1025);

    // Arrange
    t.set("integer +", 120);
    t.set("integer -", "24");
    // Act
    t.decr("integer +", 70);
    t.decr("integer -", -6);
    // Assert
    assert_eq!(t.get("integer +").unwrap(), "50");
    assert_eq!(t.get("integer -").unwrap(), "30");
}

#[test]
fn test_client_ttl_pttl_error_cases() {
    let mut t = TestClient::connect(1026);

    // Arrange
    t.set("key exists but timeout is not set", 3600);

    // Act
    let ret_timeout_not_set = (
        t.ttl("key exists but timeout is not set"),
        t.pttl("key exists but timeout is not set"),
    );
    let ret_non_existent = (
        t.ttl("non-existent key is a -2 error"),
        t.pttl("non-existent key is a -2 error"),
    );

    // Assert
    assert_eq!(t.exists("key exists but timeout is not set"), true);
    assert_eq!(ret_timeout_not_set, (-1, -1));

    assert_eq!(t.exists("non-existent key is a -2 error"), false);
    assert_eq!(ret_non_existent, (-2, -2));
}

#[test]
fn test_client_expire_pexpire() {
    let mut t = TestClient::connect(1027);

    // Arrange
    t.set("has timeout", "I will fade");

    // Act
    let dur_ms = 900_usize;
    let ret_has_timeout = (
        t.expire("has timeout", 86400_usize),
        t.pexpire("has timeout", dur_ms),
    );
    let ret_non_existent = (
        t.expire("non-existent", 78_usize),
        t.pexpire("non-existent", 12_usize),
    );

    let ret_still_alive = t.get("has timeout").unwrap();
    sleep(Duration::from_millis(dur_ms as u64));
    let ret_expired_after_sleep = t.get("has timeout");

    // Assert
    assert_eq!(ret_has_timeout, (1, 1));
    assert_eq!(ret_non_existent, (0, 0));

    assert_eq!(ret_still_alive, "I will fade");
    assert_eq!(ret_expired_after_sleep, None);
}

#[test]
fn test_client_setex_psetex() {
    let mut t = TestClient::connect(1028);

    // Arrange
    let ret_didnt_exist = (
        !t.exists("a bird in the hand is worth"),
        !t.exists("the devil's playground"),
    );

    // Act
    let dur_ms = 1000_usize;
    t.set_ex("a bird in the hand is worth", "two in the bush", 1_usize);
    t.pset_ex("the devil's playground", "an idle mind", dur_ms);

    let ret_still_alive = (
        t.get("a bird in the hand is worth").unwrap(),
        t.get("the devil's playground").unwrap(),
    );
    sleep(Duration::from_millis(dur_ms as u64));
    let ret_expired_after_sleep = (
        t.get("a bird in the hand is worth"),
        t.get("the devil's playground"),
    );

    // Assert
    assert_eq!(ret_didnt_exist, (true, true));
    assert_eq!(ret_still_alive.0, "two in the bush");
    assert_eq!(ret_still_alive.1, "an idle mind");
    assert_eq!(ret_expired_after_sleep, (None, None));
}

#[test]
fn test_client_getset() {
    let mut t = TestClient::connect(1029);

    // Arrange
    t.set("queue of one person", "James Robert");

    // Act
    let ret_getset1 = t
        .getset("queue of one person", "Patricia Jennifer")
        .unwrap();
    let ret_getset2 = t
        .getset("queue of one person", "Barbara Susan Jessica")
        .unwrap();
    let ret_get = t.get("queue of one person").unwrap();

    // Assert
    assert_eq!(ret_getset1, "James Robert");
    assert_eq!(ret_getset2, "Patricia Jennifer");
    assert_eq!(ret_get, "Barbara Susan Jessica");
}

#[test]
fn test_client_dbsize() {
    let mut t = TestClient::connect(1030);

    // Act
    let ret_dbsize1: u64 = redis::cmd("DBSIZE").query(&mut t.con).unwrap();

    t.set("1", "january");
    let ret_dbsize2: u64 = redis::cmd("DBSIZE").query(&mut t.con).unwrap();

    t.set("2", "february");
    let ret_dbsize3: u64 = redis::cmd("DBSIZE").query(&mut t.con).unwrap();

    t.del("2");
    let ret_dbsize4: u64 = redis::cmd("DBSIZE").query(&mut t.con).unwrap();

    // Assert
    assert_eq!(ret_dbsize1, 0);
    assert_eq!(ret_dbsize2, 1);
    assert_eq!(ret_dbsize3, 2);
    assert_eq!(ret_dbsize4, 1);
}

#[test]
fn test_client_mset() {
    let mut t = TestClient::connect(1031);

    // Arrange
    let id_fr = &[
        ("senin", "lundi"),
        ("rabu", "mercredi"),
        ("jumat", "vendredi"),
    ];

    // Act
    t.set_multiple(id_fr);

    let ret_days = (
        t.get("ret_days of the week"),
        t.get("senin").unwrap(),
        t.get("rabu").unwrap(),
        t.get("jumat").unwrap(),
    );

    // Assert
    assert_eq!(ret_days.0, None);
    assert_eq!(ret_days.1, "lundi");
    assert_eq!(ret_days.2, "mercredi");
    assert_eq!(ret_days.3, "vendredi");
}

#[test]
fn test_client_mset_nx() {
    let mut t = TestClient::connect(1032);

    // Arrange
    let telephony = &[
        ("l", "lima"),
        ("m", "mike"),
        ("n", "november"),
        ("o", "oscar"),
        ("p", "papa"),
    ];

    let ret_mset_nx: u8 = t.mset_nx(telephony);

    let ret_lmnop = (
        t.get("l").unwrap(),
        t.get("m").unwrap(),
        t.get("n").unwrap(),
        t.get("o").unwrap(),
        t.get("p").unwrap(),
    );

    // Act
    let ps = &[("p", "post"), ("s", "script")];
    let ret_contains_existing = t.mset_nx(ps);

    // Assert
    assert_eq!(ret_mset_nx, 1);
    assert_eq!(ret_lmnop.0, "lima");
    assert_eq!(ret_lmnop.1, "mike");
    assert_eq!(ret_lmnop.2, "november");
    assert_eq!(ret_lmnop.3, "oscar");
    assert_eq!(ret_lmnop.4, "papa");

    assert_eq!(ret_contains_existing, 0);
}

#[test]
fn test_client_hget_hset_multiple() {
    let mut t = TestClient::connect(1033);

    // Arrange
    let units = &[
        ("foot", "12 inches"),
        ("inch", "2.5 centimetres"),
        ("mile", "1.5 kilometres"),
    ];

    // Act
    t.hset_multiple("imperial", units);

    // Assert
    let ret_units = (
        t.hget("imperial", "yard"),
        t.hget("imperial", "foot").unwrap(),
        t.hget("imperial", "inch").unwrap(),
        t.hget("imperial", "mile").unwrap(),
    );

    assert_eq!(ret_units.0, None);
    assert_eq!(ret_units.1, "12 inches");
    assert_eq!(ret_units.2, "2.5 centimetres");
    assert_eq!(ret_units.3, "1.5 kilometres");
}
#[test]
fn test_client_rpush_lpush() {
    let mut t = TestClient::connect(1034);

    // Arrange
    let abc = &["a", "b", "c"];
    let def = &["d", "e", "f"];
    let ghi = &["g", "h", "i"];
    let jkl = &["j", "k", "l"];

    // Act
    let ret_rpush1 = t.rpush("list", abc);
    let ret_rpush2 = t.rpush("list", def);
    let ret_lpush1 = t.rpush("list", ghi);
    let ret_lpush2 = t.rpush("list", jkl);

    // Assert
    assert_eq!(ret_rpush1, 3);
    assert_eq!(ret_rpush2, 6);
    assert_eq!(ret_lpush1, 9);
    assert_eq!(ret_lpush2, 12);
}
#[test]
fn test_client_llen() {
    let mut t = TestClient::connect(1035);

    // Arrange
    let values = &[1, 2, 3, 4];
    t.rpush("k", values);

    // Act
    let ret_llen1 = t.llen("k");
    let ret_llen2 = t.llen("wasn't set");

    // Assert
    assert_eq!(ret_llen1, 4);
    assert_eq!(ret_llen2, 0);
}

#[test]
fn test_client_append() {
    let mut t = TestClient::connect(1036);

    // Arrange
    t.set("nb", "nota");

    // Act
    let ret_append1 = t.append("nb", "bene");
    let ret_get1 = t.get("nb").unwrap();

    let ret_append2 = t.append("behaves like SET", "hello");
    let ret_get2 = t.get("behaves like SET").unwrap();

    // Assert
    assert_eq!(ret_append1, "notabene".len());
    assert_eq!(ret_get1, "notabene");

    assert_eq!(ret_append2, "hello".len());
    assert_eq!(ret_get2, "hello");
}

fn get_redis_client_connection(port: u16) -> (Server, Connection) {
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));

    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    (server, redis_client.get_connection().unwrap())
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
