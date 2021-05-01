#[cfg(test)]
#[macro_use]
extern crate serial_test;

use std::collections::HashMap;
use std::io::{BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender};

use mpb::MPB;

use crate::command::Command;
use crate::resp::{RedisProtocolParser, RESP};

mod command;
mod resp;

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
type ReceivedDataLength = usize;
type CommandResponse = Vec<u8>;

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

fn get_bytes_from_request(stream: &TcpStream) -> ([u8; 512], usize) {
    let mut buf_reader = BufReader::new(stream);
    let mut buf = [0; 512];
    let mut buf_length = 0 as usize;

    while let Ok(s) = buf_reader.read(&mut buf) {
        buf_length += s;

        if s < 512 {
            break;
        }
    }

    (buf, buf_length)
}

fn get_command(bytes: &[u8; 512]) -> Option<Command> {
    match RedisProtocolParser::parse(bytes) {
        Ok((resp, _)) => match resp {
            RESP::Array(x) => Some(Command::parse(x)),
            _ => None,
        },
        _ => None,
    }
}

fn run_command_and_get_response(
    redisless: &Arc<Mutex<RedisLess>>,
    bytes: &[u8; 512],
) -> (Option<Command>, CommandResponse) {
    let command = get_command(bytes);

    let response = match &command {
        Some(command) => match command {
            Command::Set(k, v) => {
                unlock(redisless).set(k.as_slice(), v.as_slice());
                b"+OK\r\n".to_vec()
            }
            Command::Get(k) => match unlock(redisless).get(k.as_slice()) {
                Some(value) => {
                    let res = format!("+{}\r\n", std::str::from_utf8(value).unwrap());
                    res.as_bytes().to_vec()
                }
                None => b"$-1\r\n".to_vec(),
            },
            Command::Del(k) => {
                let total_del = unlock(redisless).del(k.as_slice());
                format!(":{}\r\n", total_del).as_bytes().to_vec()
            }
            Command::Ping => b"+PONG\r\n".to_vec(),
            Command::Quit => b"+OK\r\n".to_vec(),
            Command::NotSupported(m) => format!("-ERR {}\r\n", m).as_bytes().to_vec(),
            Command::Error(m) => format!("-ERR {}\r\n", m).as_bytes().to_vec(),
        },
        None => b"-ERR command not found\r\n".to_vec(),
    };

    (command, response)
}

fn handle_request(
    redisless: &Arc<Mutex<RedisLess>>,
    mut stream: &TcpStream,
) -> (CloseConnection, ReceivedDataLength) {
    let (buf, buf_length) = get_bytes_from_request(stream);

    match buf.get(0) {
        Some(x) if *x == 0 => {
            return (false, buf_length);
        }
        _ => {}
    }

    let (command, res) = run_command_and_get_response(redisless, &buf);

    let _ = stream.write(res.as_slice());

    match command {
        Some(command) if command == Command::Quit => (true, buf_length),
        _ => (false, buf_length),
    }
}

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
    pub fn new(redisless: RedisLess, port: u16) -> Self {
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

                                // listen incoming requests
                                for stream in listener.incoming() {
                                    match stream {
                                        Ok(tcp_stream) => {
                                            let redisless = redisless.clone();
                                            let state_recv = state_recv.clone();
                                            let state_send = state_send.clone();

                                            let _ = thread::spawn(move || {
                                                let mut last_update = SystemTime::now();

                                                loop {
                                                    let (close_connection, received_data_length) =
                                                        handle_request(&redisless, &tcp_stream);

                                                    if received_data_length > 0 {
                                                        // reset the last time we received data
                                                        last_update = SystemTime::now();
                                                    } else {
                                                        // delay the loop
                                                        thread::sleep(Duration::from_millis(10));
                                                    }

                                                    if stop_sig_received(&state_recv, &state_send)
                                                        || close_connection
                                                    {
                                                        // let's close the connection
                                                        return;
                                                    }

                                                    match last_update
                                                        .duration_since(SystemTime::now())
                                                    {
                                                        Ok(duration) => {
                                                            if duration.as_secs() >= 300 {
                                                                // close the connection after 300 secs of inactivity
                                                                return;
                                                            }
                                                        }
                                                        Err(_) => {}
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
                            Err(_) => {
                                thread::sleep(Duration::from_millis(10));
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
            match rx.recv_timeout(Duration::from_secs(5)) {
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
pub unsafe extern "C" fn redisless_server_new(port: u16) -> *mut Server {
    Box::into_raw(Box::new(Server::new(RedisLess::new(), port)))
}

#[no_mangle]
pub unsafe extern "C" fn redisless_server_free(server: *mut Server) {
    let _ = Box::from_raw(server);
}

#[no_mangle]
pub unsafe extern "C" fn redisless_server_start(server: *mut Server) -> bool {
    let server = match server.as_ref() {
        Some(server) => server,
        None => return false,
    };

    match server.start() {
        Some(server_state) => server_state == ServerState::Started,
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn redisless_server_stop(server: *mut Server) -> bool {
    let server = match server.as_ref() {
        Some(server) => server,
        None => return false,
    };

    match server.stop() {
        Some(server_state) => server_state == ServerState::Stopped,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    use redis::{Commands, RedisResult};

    use crate::{
        redisless_server_free, redisless_server_new, redisless_server_start, redisless_server_stop,
        RedisLess, Server, ServerState,
    };

    #[test]
    #[serial]
    fn start_and_stop_server_from_c_binding() {
        let port = 4444 as u16;
        let server = unsafe { redisless_server_new(port) };

        unsafe {
            assert!(redisless_server_start(server), "server didn't start");
        }

        let mut stream = TcpStream::connect(format!("localhost:{}", port)).unwrap();

        for _ in 0..29 {
            // run command `PING`
            let _ = stream.write(b"*1\r\n$4\r\nPING\r\n");
            let mut pong_res = [0; 7];
            let _ = stream.read(&mut pong_res);
            assert_eq!(pong_res, b"+PONG\r\n"[..]);

            // run command `SET mykey value`
            let _ = stream.write(b"*3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$5\r\nvalue\r\n");
            let mut set_res = [0; 5];
            let _ = stream.read(&mut set_res);
            assert_eq!(set_res, b"+OK\r\n"[..]);

            // run command `GET mykey`
            let _ = stream.write(b"*2\r\n$3\r\nGET\r\n$5\r\nmykey\r\n");
            let mut get_res = [0; 8];
            let _ = stream.read(&mut get_res);
            assert_eq!(get_res, b"+value\r\n"[..]);

            // run command `DEL mykey`
            let _ = stream.write(b"*2\r\n$3\r\nDEL\r\n$5\r\nmykey\r\n");
            let mut del_res = [0; 4];
            let _ = stream.read(&mut del_res);
            assert_eq!(del_res, b":1\r\n"[..]);
        }

        unsafe {
            assert!(redisless_server_stop(server), "server didn't stop");
            redisless_server_free(server);
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
        let port = 16379;
        let server = Server::new(RedisLess::new(), port);

        assert_eq!(server.start(), Some(ServerState::Started));

        let redis_client = redis::Client::open(format!("redis://127.0.0.1:{}/", port)).unwrap();
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
        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }

    #[test]
    fn start_and_stop_server_multiple_times() {
        let server = Server::new(RedisLess::new(), 3334);

        for _ in 0..9 {
            assert_eq!(server.start(), Some(ServerState::Started));
            assert_eq!(server.stop(), Some(ServerState::Stopped));
        }
    }
}
