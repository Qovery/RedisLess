use std::net::SocketAddr;

use rand::rngs::OsRng;

use raft::log::memory::InMemoryLog;
use raft::node::Node;

use crate::cluster::peer::Peers;

type RaftNode = Node<InMemoryLog, OsRng, String>;

pub struct ClusterNode {
    node: RaftNode,
    peers: Peers,
    listening_socket_addr: SocketAddr,
    listener_started: bool,
}

impl ClusterNode {
    pub fn new(node: RaftNode, peers: Peers, listening_socket_addr: SocketAddr) -> Self {
        ClusterNode {
            node,
            peers,
            listening_socket_addr,
            listener_started: false,
        }
    }

    // start TCP socket listener to handle incoming message from peers
    pub fn start_listener(&self) {
        if self.listener_started {
            return;
        }

        // TODO
    }

    // stop TCP socket listener to handle incoming message from peers
    pub fn stop_listener(&self) {
        if !self.listener_started {
            return;
        }

        // TODO
    }
}
