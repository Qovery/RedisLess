use std::io::{BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender};
use mpb::MPB;

use crate::command::Command;
use crate::protocol;
use crate::protocol::{RedisProtocolParser, RESP};
use crate::storage::Storage;

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
        let state_send = self.server_state_bus.tx();
        let state_recv = self.server_state_bus.rx();

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

fn unlock<T: Storage>(storage: &Arc<Mutex<T>>) -> MutexGuard<T> {
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

fn run_command_and_get_response<T: Storage>(
    storage: &Arc<Mutex<T>>,
    bytes: &[u8; 512],
) -> (Option<Command>, CommandResponse) {
    let command = get_command(bytes);

    let response = match &command {
        Some(command) => match command {
            Command::Set(k, v) => {
                unlock(storage).set(k.as_slice(), v.as_slice());
                protocol::OK.to_vec()
            }
            Command::Get(k) => match unlock(storage).get(k.as_slice()) {
                Some(value) => {
                    let res = format!("+{}\r\n", std::str::from_utf8(value).unwrap());
                    res.as_bytes().to_vec()
                }
                None => protocol::NIL.to_vec(),
            },
            Command::Del(k) => {
                let total_del = unlock(storage).del(k.as_slice());
                format!(":{}\r\n", total_del).as_bytes().to_vec()
            }
            Command::Incr(k) => match unlock(storage).get(k.as_slice()) {
                Some(value) => {
                    if let Ok(mut int_val) = std::str::from_utf8(value).unwrap().parse::<i64>() {
                        int_val += 1;
                        let new_value = int_val.to_string().into_bytes();
                        unlock(storage).set(k.as_slice(), new_value.as_slice());

                        format!(":{}\r\n", int_val).as_bytes().to_vec()
                    } else {
                        b"-WRONGTYPE Operation against a key holding the wrong kind of value}}"
                            .to_vec()
                    }
                }
                None => {
                    let val = "1";
                    unlock(storage).set(k, val.as_bytes());
                    format!(":{}\r\n", val).as_bytes().to_vec()
                }
            },
            Command::Info => protocol::EMPTY_LIST.to_vec(), // TODO change with some real info?
            Command::Ping => protocol::PONG.to_vec(),
            Command::Quit => protocol::OK.to_vec(),
            Command::NotSupported(m) => format!("-ERR {}\r\n", m).as_bytes().to_vec(),
            Command::Error(m) => format!("-ERR {}\r\n", m).as_bytes().to_vec(),
        },
        None => b"-ERR command not found\r\n".to_vec(),
    };

    (command, response)
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
    addr: &String,
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

                        match last_update.duration_since(SystemTime::now()) {
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

#[cfg(test)]
mod tests {
    use redis::{cmd, Commands, RedisResult};

    use crate::server::ServerState;
    use crate::storage::in_memory::InMemoryStorage;
    use crate::Server;

    #[test]
    #[serial]
    fn test_redis_implementation() {
        let port = 16379;
        let server = Server::new(InMemoryStorage::new(), port);

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

        let _: () = con.set("intkey", "10").unwrap();
        con.send_packed_command(cmd("INCR").arg("intkey").get_packed_command().as_slice())
            .unwrap();
        let x: String = con.get("intkey").unwrap();
        assert_eq!(x, "11");

        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }

    #[test]
    #[serial]
    fn start_and_stop_server() {
        let server = Server::new(InMemoryStorage::new(), 3333);
        assert_eq!(server.start(), Some(ServerState::Started));
        assert_eq!(server.stop(), Some(ServerState::Stopped));
    }

    #[test]
    fn start_and_stop_server_multiple_times() {
        let server = Server::new(InMemoryStorage::new(), 3334);

        for _ in 0..9 {
            assert_eq!(server.start(), Some(ServerState::Started));
            assert_eq!(server.stop(), Some(ServerState::Stopped));
        }
    }
}
