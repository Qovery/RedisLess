use crate::server::ServerState;
use crate::storage::in_memory::InMemoryStorage;
use crate::Server;
use redis::{Commands, Connection, ToRedisArgs};
use rstest::*;
use std::fmt::Debug;
use CommandDefn::{IntArg, StringArg};

fn get_server_connection(port: u16) -> (Server, Connection) {
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));

    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    (server, redis_client.get_connection().unwrap())
}

struct TestConnection {
    server: Server,
    con: Connection,
}
impl TestConnection {
    fn start(port: u16) -> Self {
        let (server, con) = get_server_connection(port);
        TestConnection { server, con }
    }
    fn redis_set<K: ToRedisArgs, V: ToRedisArgs>(&mut self, k: K, v: V) {
        let _: () = self.con.set(k, v).unwrap();
    }
    fn redis_incr<K: ToRedisArgs, V: ToRedisArgs>(&mut self, k: K, v: V) {
        let _: () = self.con.incr(k, v).unwrap();
    }
    fn test_redis_get<K: ToRedisArgs, V: ToString + Debug>(&mut self, k: K, v: V) {
        let res: String = self.con.get(k).unwrap();
        assert_eq!(res, v.to_string());
    }
    fn run(&mut self, case: Vec<Vec<CommandDefn>>) {
        for defn in case {
            if defn.len() > 0 {
                match defn[0] {
                    StringArg("set") => match (defn[1], defn[2]) {
                        (IntArg(k), IntArg(v)) => self.redis_set(k, v),
                        (IntArg(k), StringArg(v)) => self.redis_set(k, v),
                        (StringArg(k), IntArg(v)) => self.redis_set(k, v),
                        (StringArg(k), StringArg(v)) => self.redis_set(k, v),
                    },
                    StringArg("incr") => match (defn[1], defn[2]) {
                        (IntArg(k), IntArg(v)) => self.redis_incr(k, v),
                        (IntArg(k), StringArg(v)) => self.redis_incr(k, v),
                        (StringArg(k), IntArg(v)) => self.redis_incr(k, v),
                        (StringArg(k), StringArg(v)) => self.redis_incr(k, v),
                    },
                    StringArg("test_get") => match (defn[1], defn[2]) {
                        (IntArg(k), IntArg(v)) => self.test_redis_get(k, v),
                        (IntArg(k), StringArg(v)) => self.test_redis_get(k, v),
                        (StringArg(k), IntArg(v)) => self.test_redis_get(k, v),
                        (StringArg(k), StringArg(v)) => self.test_redis_get(k, v),
                    },
                    _ => {}
                }
            } else {
                break;
            }
        }
        self.stop();
    }
    fn stop(&mut self) {
        assert_eq!(self.server.stop(), Some(ServerState::Stopped));
    }
}

#[derive(Clone, Copy)]
enum CommandDefn<'a> {
    StringArg(&'a str),
    IntArg(i64),
}
impl From<i64> for CommandDefn<'_> {
    fn from(n: i64) -> Self {
        CommandDefn::IntArg(n)
    }
}
impl<'a> From<&'a str> for CommandDefn<'a> {
    fn from(n: &'a str) -> Self {
        CommandDefn::StringArg(n)
    }
}

#[macro_export]
macro_rules! command_defn {
    ( $x0:expr $(, $x:expr )+ ) => {{
        let mut v: Vec<CommandDefn> = Vec::new();
        v.push( $x0.into() );
        $(
            v.push( $x.into() );
        )*
        v
    }};
}

#[rstest]
#[case::set_existent_key(
    3001,
    vec![
        command_defn!("set", 12, "5"),
        command_defn!("set", "12", 1200),
        command_defn!("test_get", 12, "1200"),
    ]
)]
#[case::incr_by_1(
    3002,
    vec![
        command_defn!["set", "12", "34"],
        command_defn!["incr", "12", "1"],
        command_defn!["test_get", "12", "35"],
    ]
)]
fn test_redis_client(#[case] port: u16, #[case] command_defns: Vec<Vec<CommandDefn>>) {
    let mut t = TestConnection::start(port);
    t.run(command_defns);
}
