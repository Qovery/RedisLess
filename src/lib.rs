use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io::Error;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;

mod redis_protocol;

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

fn handle_request(redisless: &RedisLess, mut stream: &TcpStream) {
    // TODO
}

#[repr(C)]
pub struct Server {
    redisless: Arc<RedisLess>,
    send_state_ch: Sender<ServerState>,
    recv_state_ch: Receiver<ServerState>,
}

enum ServerState {
    Start,
    Stop,
}

impl Server {
    pub fn new(redisless: RedisLess) -> Self {
        let (send_state_ch, recv_state_ch) = unbounded::<ServerState>();

        let s = Server {
            redisless: Arc::new(redisless),
            send_state_ch,
            recv_state_ch,
        };

        // TODO export conf
        s._init_configuration("0.0.0.0:16379");

        s
    }

    pub fn _init_configuration<A: Into<String>>(&self, addr: A) {
        let addr = Arc::new(addr.into());
        let redisless = self.redisless.clone();
        let recv = self.recv_state_ch.clone();

        thread::spawn(move || {
            for server_state in recv {
                let addr = addr.clone();
                let redisless = redisless.clone();

                match server_state {
                    ServerState::Start => {
                        let _ = thread::spawn(move || match TcpListener::bind(addr.as_str()) {
                            Ok(listener) => {
                                // listen incoming requests
                                for stream in listener.incoming() {
                                    let redisless = redisless.clone();

                                    let _ = thread::spawn(move || match stream {
                                        Ok(tcp_stream) => loop {
                                            handle_request(redisless.as_ref(), &tcp_stream);
                                        },
                                        Err(err) => {
                                            println!("{:?}", err);
                                        }
                                    });
                                }
                            }
                            Err(err) => {
                                println!("{:?}", err);
                            }
                        });
                    }
                    ServerState::Stop => {
                        // TODO implement
                    }
                }
            }
        });
    }

    /// start server
    pub fn start(&self) {
        self.send_state_ch.send(ServerState::Start);
    }

    /// stop server
    pub fn stop(&self) {
        self.send_state_ch.send(ServerState::Stop);
    }
}

#[no_mangle]
pub extern "C" fn redisless_new() -> *mut RedisLess {
    Box::into_raw(Box::new(RedisLess::new()))
}

#[no_mangle]
pub extern "C" fn redisless_server_new(redisless: RedisLess) -> *const Server {
    Box::into_raw(Box::new(Server::new(redisless)))
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
    use crate::RedisLess;

    #[test]
    fn test_set_get_and_del() {
        let mut redisless = RedisLess::new();
        redisless.set(b"key", b"xxx");
        assert_eq!(redisless.get(b"key"), Some(&b"xxx"[..]));
        assert_eq!(redisless.del(b"key"), 1);
        assert_eq!(redisless.del(b"key"), 0);
        assert_eq!(redisless.get(b"does not exist"), None);
    }
}
