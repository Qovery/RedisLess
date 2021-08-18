use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender};
use mpb::MPB;
use rayon::ThreadPool;
use uuid::Uuid;

use util::*;

use crate::cluster::peer::{Peer, PeersDiscovery, DEFAULT_NODE_LISTENING_PORT};
use crate::storage::Storage;

#[cfg(test)]
mod tests;

mod util;

type CloseConnection = bool;
type ReceivedDataLength = usize;

pub struct Server {
    server_state_bus: MPB<ServerState>,
    cluster_options: ServerClusterOptions,
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

#[derive(Debug)]
pub struct ServerClusterOptions {
    group_id: String,
    peers_discovery: PeersDiscovery,
    listening_socket_addr: SocketAddr,
}

impl ServerClusterOptions {
    pub fn new(
        group_id: String,
        peers_discovery: PeersDiscovery,
        listening_socket_addr: SocketAddr,
    ) -> Self {
        ServerClusterOptions {
            group_id,
            peers_discovery,
            listening_socket_addr,
        }
    }
}

impl Default for ServerClusterOptions {
    fn default() -> Self {
        ServerClusterOptions {
            group_id: String::from("primary"),
            peers_discovery: PeersDiscovery::Automatic(DEFAULT_NODE_LISTENING_PORT),
            listening_socket_addr: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                DEFAULT_NODE_LISTENING_PORT,
            ),
        }
    }
}

impl Server {
    pub fn new<T: Storage + Send + 'static>(storage: T, port: u16) -> Self {
        Server::new_with_cluster_options(storage, ServerClusterOptions::default(), port)
    }

    pub fn new_with_cluster_options<T: Storage + Send + 'static>(
        storage: T,
        cluster_options: ServerClusterOptions,
        port: u16,
    ) -> Self {
        let s = Server {
            server_state_bus: MPB::new(),
            cluster_options,
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

        let id = Uuid::new_v4();
        let peer = Peer::new(
            id.to_string(),
            PeersDiscovery::Automatic(self.cluster_options.listening_socket_addr.port()),
            self.cluster_options.listening_socket_addr,
        );

        let mut cluster_node = peer.into_cluster_node();

        let _ = thread::spawn(move || {
            let addr = addr;
            let storage = Arc::new(Mutex::new(storage));

            loop {
                if let Ok(server_state) = state_recv.recv() {
                    if server_state == ServerState::Start {
                        // start local RESP server
                        start_server(&addr, &state_send, &state_recv, &storage);

                        // start current node listener
                        cluster_node.start_listener();
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
                handle_tcp_stream(tcp_stream, &thread_pool, state_send, state_recv, storage);
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                break;
            }
        }

        if stop_sig_received(state_recv, state_send) {
            // let's gracefully shutdown the server
            break;
        }
    }
}

fn handle_tcp_stream<T: Storage + Send + 'static>(
    tcp_stream: TcpStream,
    thread_pool: &ThreadPool,
    state_send: &Sender<ServerState>,
    state_recv: &Receiver<ServerState>,
    storage: &Arc<Mutex<T>>,
) {
    let storage = storage.clone();
    let state_recv = state_recv.clone();
    let state_send = state_send.clone();

    let _ = thread_pool.spawn(move || {
        let mut last_update = SystemTime::now();

        loop {
            let (close_connection, received_data_length) = handle_request(&storage, &tcp_stream);

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
