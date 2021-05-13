use std::io::{BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender};
use mpb::MPB;

use crate::protocol;
use crate::protocol::{RedisProtocolParser, Resp};
use crate::storage::Storage;
use crate::{command::Command, protocol::error::RedisCommandError};

type CloseConnection = bool;
type ReceivedDataLength = usize;
type CommandResponse = Vec<u8>;

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
    pub fn new<T: Storage + Send + 'static>(storage: T, port: u16) -> Self {
        let s = Server {
            server_state_bus: MPB::new(),
        };

        s._init_configuration(format!("0.0.0.0:{}", port), storage);
        s
    }

    fn _init_configuration<A: Into<String>, T: Storage + Send + 'static>(
        &self,
        addr: A,
        storage: T,
    ) {
        let addr = addr.into();
        let state_send = self.server_state_bus.sender();
        let state_recv = self.server_state_bus.receiver();

        let _ = thread::spawn(move || {
            let addr = addr;
            let storage = Arc::new(Mutex::new(storage));

            loop {
                if let Ok(server_state) = state_recv.recv() {
                    if server_state == ServerState::Start {
                        start_server(&addr, &state_send, &state_recv, &storage);
                    }
                }
            }
        });
    }

    fn change_state(&self, change_to: ServerState) -> Option<ServerState> {
        let send_state_ch = self.server_state_bus.sender();

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
        let receiver = self.server_state_bus.receiver(); // TODO cache receiver to reuse it?

        while let Ok(server_state) = receiver.recv_timeout(Duration::from_secs(5)) {
            if server_state == post_change_to_state {
                return Some(server_state);
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

fn lock_then_release<T: Storage>(storage: &Arc<Mutex<T>>) -> MutexGuard<T> {
    loop {
        match storage.lock() {
            Ok(storage) => {
                return storage;
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn get_bytes_from_request(stream: &TcpStream) -> ([u8; 512], usize) {
    let mut buf_reader = BufReader::new(stream);
    let mut buf = [0; 512];
    let mut buf_length = 0_usize;

    while let Ok(s) = buf_reader.read(&mut buf) {
        buf_length += s;

        if s < 512 {
            break;
        }
    }

    (buf, buf_length)
}

fn get_command(bytes: &[u8; 512]) -> Result<Command, RedisCommandError> {
    match RedisProtocolParser::parse(bytes) {
        Ok((Resp::Array(v), _)) => match Command::parse(v) {
            Ok(command) => Ok(command),
            Err(err) => Err(err),
        },
        Err(err) => Err(RedisCommandError::ProtocolParse(err)),
        _ => Err(RedisCommandError::CommandNotFound),
    }
}

fn run_command_and_get_response<T: Storage>(
    storage: &Arc<Mutex<T>>,
    bytes: &[u8; 512],
) -> (Option<Command>, CommandResponse) {
    let command = get_command(bytes);

    let response = match &command {
        Ok(command) => match command {
            Command::Set(k, v) => {
                lock_then_release(storage).write(k.as_slice(), v.as_slice());
                protocol::OK.to_vec()
            }
            Command::Setex(k, expiry, v) => {
                let mut storage = lock_then_release(storage);

                storage.write(k.as_slice(), v.as_slice());
                storage.expire(k.as_slice(), *expiry);

                protocol::OK.to_vec()
            }
            Command::Setnx(k, v) => {
                let mut storage = lock_then_release(storage);
                match storage.contains(k) {
                    // Key exists, will not re set key
                    true => b":0\r\n".to_vec(),
                    // Key does not exist, will set key
                    false => {
                        storage.write(k, v);
                        b":1\r\n".to_vec()
                    }
                }
            }
            Command::MSetnx(items) => {
                // Either set all or not set any at all if any already exist
                let mut storage = lock_then_release(storage);
                match items.iter().all(|(key, _)| !storage.contains(key)) {
                    // None of the keys already exist in the storage
                    true => {
                        items.iter().for_each(|(k, v)| storage.write(k, v));
                        b":1\r\n".to_vec()
                    }
                    // Some key exists, don't write any of the keys
                    false => b":0\r\n".to_vec(),
                }
            }
            Command::Expire(k, expiry) | Command::PExpire(k, expiry) => {
                let v = lock_then_release(storage).expire(k.as_slice(), *expiry);
                format!(":{}\r\n", v).as_bytes().to_vec()
            }
            Command::Get(k) => match lock_then_release(storage).read(k.as_slice()) {
                Some(value) => {
                    let res = format!("+{}\r\n", std::str::from_utf8(value).unwrap());
                    res.as_bytes().to_vec()
                }
                None => protocol::NIL.to_vec(),
            },
            Command::GetSet(k, v) => {
                let mut storage = lock_then_release(storage);

                let response = match storage.read(k.as_slice()) {
                    Some(value) => {
                        let res = format!("+{}\r\n", std::str::from_utf8(value).unwrap());
                        res.as_bytes().to_vec()
                    }
                    None => protocol::NIL.to_vec(),
                };
                storage.write(k.as_slice(), v.as_slice());
                response
            }
            Command::Del(k) => {
                let total_del = lock_then_release(storage).remove(k.as_slice());
                format!(":{}\r\n", total_del).as_bytes().to_vec()
            }
            Command::Incr(k) => {
                let mut storage = lock_then_release(storage);

                match storage.read(k.as_slice()) {
                    Some(value) => {
                        if let Ok(mut int_val) = std::str::from_utf8(value).unwrap().parse::<i64>()
                        {
                            int_val += 1;
                            let new_value = int_val.to_string().into_bytes();
                            storage.write(k.as_slice(), new_value.as_slice());

                            format!(":{}\r\n", int_val).as_bytes().to_vec()
                        } else {
                            b"-WRONGTYPE Operation against a key holding the wrong kind of value}}"
                                .to_vec()
                        }
                    }
                    None => {
                        let val = "1";
                        storage.write(k, val.as_bytes());
                        format!(":{}\r\n", val).as_bytes().to_vec()
                    }
                }
            }
            Command::Exists(k) => {
                let exists = lock_then_release(storage).contains(k);
                let exists: u32 = match exists {
                    true => 1,
                    false => 0,
                };
                format!(":{}\r\n", exists).as_bytes().to_vec()
            }
            Command::Info => protocol::EMPTY_LIST.to_vec(), // TODO change with some real info?
            Command::Ping => protocol::PONG.to_vec(),
            Command::Quit => protocol::OK.to_vec(),
        },
        Err(err) => format!("-ERR {}\r\n", err).as_bytes().to_vec(),
    };

    (command.ok(), response)
}

fn handle_request<T: Storage>(
    storage: &Arc<Mutex<T>>,
    mut stream: &TcpStream,
) -> (CloseConnection, ReceivedDataLength) {
    let (buf, buf_length) = get_bytes_from_request(stream);

    match buf.get(0) {
        Some(x) if *x == 0 => {
            return (false, buf_length);
        }
        _ => {}
    }

    let (command, res) = run_command_and_get_response(storage, &buf);

    let _ = stream.write(res.as_slice());

    match command {
        Some(command) if command == Command::Quit => (true, buf_length),
        _ => (false, buf_length),
    }
}

fn start_server<T: Storage + Send + 'static>(
    addr: &str,
    state_send: &Sender<ServerState>,
    state_recv: &Receiver<ServerState>,
    storage: &Arc<Mutex<T>>,
) {
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => {
            // notify that the server has been started
            let _ = state_send.send(ServerState::Started);
            let _ = listener.set_nonblocking(true);
            listener
        }
        Err(_) => {
            thread::sleep(Duration::from_millis(10));
            return;
        }
    };

    let thread_pool = match rayon::ThreadPoolBuilder::new()
        .thread_name(|_| "request handler".to_string())
        .num_threads(4)
        .build()
    {
        Ok(pool) => pool,
        Err(err) => {
            panic!("{:?}", err);
        }
    };

    // listen incoming requests
    for stream in listener.incoming() {
        match stream {
            Ok(tcp_stream) => {
                let storage = storage.clone();
                let state_recv = state_recv.clone();
                let state_send = state_send.clone();

                let _ = thread_pool.spawn(move || {
                    let mut last_update = SystemTime::now();

                    loop {
                        let (close_connection, received_data_length) =
                            handle_request(&storage, &tcp_stream);

                        if received_data_length > 0 {
                            // reset the last time we received data
                            last_update = SystemTime::now();
                        } else {
                            // delay the loop
                            thread::sleep(Duration::from_millis(10));
                        }

                        if stop_sig_received(&state_recv, &state_send) || close_connection {
                            // let's close the connection
                            return;
                        }

                        if let Ok(duration) = last_update.duration_since(SystemTime::now()) {
                            if duration.as_secs() >= 300 {
                                // close the connection after 300 secs of inactivity
                                return;
                            }
                        }

                        if close_connection {
                            return;
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
            // let's gracefully shutdown the server
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use redis::{cmd, Commands, RedisResult};

    use crate::server::ServerState;
    use crate::storage::in_memory::InMemoryStorage;
    use crate::Server;

    #[test]
    #[serial]
    fn test_redis_implementation() {
        let port = 3340;
        let server = Server::new(InMemoryStorage::new(), port);

        assert_eq!(server.start(), Some(ServerState::Started)); // this fails

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

        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }

    #[test]
    #[serial]
    fn expire() {
        let port = 3340;
        let server = Server::new(InMemoryStorage::new(), port);
        assert_eq!(server.start(), Some(ServerState::Started)); // this doesnt fail ??

        let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
        let mut con = redis_client.get_connection().unwrap();

        let duration: usize = 2387;
        let _: () = con.set("key2", "value2").unwrap();
        let x: String = con.get("key2").unwrap();
        assert_eq!(x, "value2");

        let ret_val: u32 = con.pexpire("key2", duration).unwrap();
        assert_eq!(ret_val, 1);

        sleep(Duration::from_millis(duration as u64));
        let x: Option<String> = con.get("key2").ok();
        assert_eq!(x, None);

        let duration: usize = 2;
        let _: () = con.set_ex("key", "value", duration).unwrap();
        let x: String = con.get("key").unwrap();
        assert_eq!(x, "value");

        sleep(Duration::from_secs(duration as u64));
        let x: Option<String> = con.get("key").ok();
        assert_eq!(x, None);

        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }

    #[test]
    #[serial]
    fn get_set() {
        let port = 3340;
        let server = Server::new(InMemoryStorage::new(), port);
        assert_eq!(server.start(), Some(ServerState::Started)); // this doesnt fail ??

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
    fn mset_nx() {
        // make these first 5 lines into a macro?
        let port = 3340;
        let server = Server::new(InMemoryStorage::new(), port);
        assert_eq!(server.start(), Some(ServerState::Started)); // this doesnt fail ??
        let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
        let mut con = redis_client.get_connection().unwrap();

        // what should happen if keys are repeated in the request? currently the last one is the one  that is set
        // i think the official redis implementation behaves this way as well?
        let key_value_pairs = &[("key1", "val1"), ("key2", "val2"), ("key3", "val3")][..];

        let x = con
            .mset_nx::<&'static str, &'static str, u32>(key_value_pairs)
            .unwrap();
        assert_eq!(x, 1);
        let x: String = con.get("key1").unwrap();
        assert_eq!(x, "val1");
        let x: String = con.get("key2").unwrap();
        assert_eq!(x, "val2");
        let x: String = con.get("key3").unwrap();
        assert_eq!(x, "val3");

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
}
