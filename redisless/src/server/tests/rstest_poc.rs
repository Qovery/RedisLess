use redis::ToRedisArgs;
use rstest::*;
use redis::{Commands, Connection, RedisResult};
use std::{thread::sleep, time::Duration};

use crate::server::ServerState;
use crate::storage::in_memory::InMemoryStorage;
use crate::Server;

fn get_server_connection(port: u16) -> (Server, Connection) {
    let server = Server::new(InMemoryStorage::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));

    let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
    (server, redis_client.get_connection().unwrap())
}

fn con_set(key: u32, value: u32) -> Box<dyn Fn(&mut Connection)> {
    let clsr = move |con: &mut Connection| -> () {
        con.set(key, value).unwrap()
    };
    Box::new(clsr)
}
fn con_incr(key: u32, delta: u32) -> Box<dyn Fn(&mut Connection)> {
    let clsr = move |con: &mut Connection| -> () {
        con.incr(key, delta).unwrap()
    };
    Box::new(clsr)
}
fn assert_con_exists(key: u32, expected: bool) -> Box<dyn Fn(&mut Connection)> {
    let clsr = move |con: &mut Connection| -> () {
        let res: bool = con.exists(key).unwrap();
        assert_eq!(res, expected);
    };
    Box::new(clsr)
}
fn assert_con_get(key: u32, expected: u32) -> Box<dyn Fn(&mut Connection)> {
    let clsr = move |con: &mut Connection| -> () {
        let res: u32 = con.get(key).unwrap();
        assert_eq!(res, expected);
    };
    Box::new(clsr)
}
#[rstest]
#[case::incr_by_1(
    3001_u16,
    vec![ 
        // precondition:
        con_set(23, 45),
        // test subject:
        con_incr(23, 1),
        // assertions:
        assert_con_exists(23, true),
        assert_con_get(23, 46),
    ]
)]
fn redis_client(
    #[case] port: u16,
    #[case] commands: Vec<Box<dyn for<'r> Fn(&'r mut Connection)>>,
){
    let (server, mut con) = get_server_connection(port);
    commands.iter().for_each(|f| f(&mut con));
    assert_eq!(server.stop(), Some(ServerState::Stopped));
}
