#[cfg(test)]
#[macro_use]
extern crate serial_test;

use std::collections::HashMap;
use std::io::{BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

use mpb::MPB;

use crate::command::Command;
use crate::resp::{RedisProtocolParser, RESP};

mod command;
mod resp;

#[repr(C)]
pub struct RedisLess {
    data_mapper: HashMap<Vec<u8>, DataType>,
    string_store: HashMap<Vec<u8>, Vec<u8>>,
}

enum DataType {
    String,
    List,
    Set,
    Hash,
}

impl RedisLess {
    pub fn new() -> Self {
        RedisLess {
            data_mapper: HashMap::new(),
            string_store: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        self.data_mapper.insert(key.to_vec(), DataType::String);
        self.string_store.insert(key.to_vec(), value.to_vec());
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.string_store.get(key).map(|v| &v[..])
    }

    pub fn del(&mut self, key: &[u8]) -> u32 {
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

fn stop_sig_received(recv: &Receiver<ServerState>, sender: &Sender<ServerState>) -> bool {
    if let Ok(recv_state) = recv.try_recv() {
        if recv_state == ServerState::Stop {
            // notify that the server has been stopped
            let _ = sender.send(ServerState::Stopped);
            return true;
        }
    }

    false
}

type CloseConnection = bool;

fn unlock(redisless: &Arc<Mutex<RedisLess>>) -> MutexGuard<RedisLess> {
    loop {
        match redisless.lock() {
            Ok(redisless) => {
                return redisless;
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn handle_request(redisless: &Arc<Mutex<RedisLess>>, mut stream: &TcpStream) -> CloseConnection {
    let mut buf_reader = BufReader::new(stream);
    let mut buf = [0; 512];

    while let Ok(s) = buf_reader.read(&mut buf) {
        if s < 512 {
            break;
        }
    }

    match buf.get(0) {
        Some(x) if *x == 0 => {
            return false;
        }
        _ => {}
    }

    match RedisProtocolParser::parse(&buf) {
        Ok((resp, _)) => match resp {
            RESP::Array(x) => match Command::parse(x) {
                Command::Set(k, v) => {
                    unlock(redisless).set(k.as_slice(), v.as_slice());
                    let _ = stream.write(b"+OK\r\n");
                }
                Command::Get(k) => {
                    if let Some(value) = unlock(redisless).get(k.as_slice()) {
                        let res = format!("+{}\r\n", std::str::from_utf8(value).unwrap());
                        let _ = stream.write(res.as_bytes());
                        return false;
                    };

                    let _ = stream.write(b"$-1\r\n");
                }
                Command::Del(k) => {
                    let total_del = unlock(redisless).del(k.as_slice());
                    let res = format!(":{}\r\n", total_del);
                    let _ = stream.write(res.as_bytes());
                }
                Command::Ping => {
                    let _ = stream.write(b"+PONG\r\n");
                }
                Command::Quit => {
                    let _ = stream.write(b"+OK\r\n");
                    return true;
                }
                Command::NotSupported(m) => {
                    let _ = stream.write(format!("-ERR {}\r\n", m).as_bytes());
                }
                Command::Error(m) => {
                    let _ = stream.write(format!("-ERR {}\r\n", m).as_bytes());
                }
            },
            _ => {}
        },
        _ => {}
    };

    false
}

#[repr(C)]
pub struct Server {
    server_state_bus: MPB<ServerState>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ServerState {
    Start,
    Started,
    Stop,
    Stopped,
    Timeout,
    Error(String),
}

impl Server {
    pub fn new(redisless: RedisLess, port: u32) -> Self {
        let s = Server {
            server_state_bus: MPB::new(),
        };

        s._init_configuration(format!("0.0.0.0:{}", port), redisless);
        s
    }

    fn _init_configuration<A: Into<String>>(&self, addr: A, redisless: RedisLess) {
        let addr = addr.into();
        let state_send = self.server_state_bus.tx();
        let state_recv = self.server_state_bus.rx();

        let _ = thread::spawn(move || {
            let addr = addr;
            let redisless = Arc::new(Mutex::new(redisless));

            loop {
                if let Ok(server_state) = state_recv.recv() {
                    if server_state == ServerState::Start {
                        match TcpListener::bind(addr.as_str()) {
                            Ok(listener) => {
                                // notify that the server has been started
                                let _ = state_send.send(ServerState::Started);
                                let _ = listener.set_nonblocking(true);
                                let _ = listener.set_ttl(60);

                                // listen incoming requests
                                for stream in listener.incoming() {
                                    match stream {
                                        Ok(tcp_stream) => {
                                            let redisless = redisless.clone();
                                            let state_recv = state_recv.clone();
                                            let state_send = state_send.clone();

                                            let _ = thread::spawn(move || {
                                                loop {
                                                    let close_connection =
                                                        handle_request(&redisless, &tcp_stream);

                                                    if stop_sig_received(&state_recv, &state_send) {
                                                        // then we can gracefully shutdown the server
                                                        return;
                                                    }

                                                    if close_connection {
                                                        break;
                                                    }
                                                }
                                            });
                                        }
                                        Err(err) if err.kind() == ErrorKind::WouldBlock => {
                                            thread::sleep(Duration::from_millis(10));
                                        }
                                        Err(_) => {
                                            break;
                                        }
                                    }

                                    if stop_sig_received(&state_recv, &state_send) {
                                        return;
                                    }
                                }
                            }
                            Err(err) => {
                                let _ = state_send.send(ServerState::Error(format!("{:?}", err)));
                            }
                        };
                    }
                }
            }
        });
    }

    fn change_state(&self, change_to: ServerState) -> Option<ServerState> {
        let send_state_ch = self.server_state_bus.tx();

        let post_change_to_state = match change_to {
            ServerState::Start => ServerState::Started,
            ServerState::Stop => ServerState::Stopped,
            ServerState::Started
            | ServerState::Stopped
            | ServerState::Timeout
            | ServerState::Error(_) => return None,
        };

        let _ = thread::spawn(move || {
            let _ = thread::sleep(Duration::from_millis(100));
            let _ = send_state_ch.send(change_to);
        });

        // wait for changing state
        let rx = self.server_state_bus.rx(); // TODO cache rx to reuse it?

        loop {
            match rx.recv_timeout(Duration::from_secs(10)) {
                Ok(server_state) => {
                    if server_state == post_change_to_state {
                        return Some(server_state);
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }

        Some(ServerState::Timeout)
    }

    /// start server
    pub fn start(&self) -> Option<ServerState> {
        self.change_state(ServerState::Start)
    }

    /// stop server
    pub fn stop(&self) -> Option<ServerState> {
        self.change_state(ServerState::Stop)
    }
}

#[no_mangle]
pub extern "C" fn redisless_new() -> *const RedisLess {
    Box::into_raw(Box::new(RedisLess::new()))
}

#[no_mangle]
pub extern "C" fn redisless_server_new(redisless: RedisLess) -> *const Server {
    Box::into_raw(Box::new(Server::new(redisless, 16379)))
}

#[no_mangle]
pub extern "C" fn redisless_server_start(server: &Server) {
    server.start();
}

#[no_mangle]
pub extern "C" fn redisless_server_stop(server: &Server) {
    server.stop();
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use redis::{Commands, RedisResult};

    use crate::{
        redisless_new, redisless_server_new, redisless_server_start, redisless_server_stop,
        RedisLess, Server, ServerState,
    };

    #[test]
    #[serial]
    fn start_and_stop_server_from_c_binding() {
        let redisless = unsafe { std::ptr::read(redisless_new()) };
        let server = redisless_server_new(redisless);

        unsafe {
            redisless_server_start(&*server);
        }

        unsafe {
            redisless_server_stop(&*server);
        }
    }

    #[test]
    fn test_set_get_and_del() {
        let mut redisless = RedisLess::new();
        redisless.set(b"key", b"xxx");
        assert_eq!(redisless.get(b"key"), Some(&b"xxx"[..]));
        assert_eq!(redisless.del(b"key"), 1);
        assert_eq!(redisless.del(b"key"), 0);
        assert_eq!(redisless.get(b"does not exist"), None);
    }

    #[test]
    #[serial]
    fn test_redis_implementation() {
        let server = Server::new(RedisLess::new(), 16379);

        assert_eq!(server.start(), Some(ServerState::Started));

        let redis_client = redis::Client::open("redis://127.0.0.1:16379/").unwrap();
        let mut con = redis_client.get_connection().unwrap();

        let _: () = con.set("key", "value").unwrap();
        let x: String = con.get("key").unwrap();
        assert_eq!(x, "value");

        let x: RedisResult<String> = con.get("not-existing-key");
        assert_eq!(x.is_err(), true);

        let x: u32 = con.del("key").unwrap();
        assert_eq!(x, 1);

        let x: u32 = con.del("key").unwrap();
        assert_eq!(x, 0);

        let _: () = con.set("key2", "value2").unwrap();
        let x: String = con.get("key2").unwrap();
        assert_eq!(x, "value2");

        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }

    #[test]
    #[serial]
    fn start_and_stop_server() {
        let server = Server::new(RedisLess::new(), 3333);
        assert_eq!(server.start(), Some(ServerState::Started));

        // thread::sleep(Duration::from_secs(3600));

        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }
}
