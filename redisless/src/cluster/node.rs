use std::net::SocketAddr;

use rand::rngs::OsRng;

use raft::log::memory::InMemoryLog;
use raft::node::Node;

type RaftNode = Node<InMemoryLog, OsRng, String>;

pub struct ClusterNode {
    node: RaftNode,
    socket_addr: SocketAddr,
}

impl ClusterNode {
    pub fn new(node: RaftNode, socket_addr: SocketAddr) -> Self {
        ClusterNode { node, socket_addr }
    }

    // start TCP socket listener to handle incoming message from peers
    pub fn start_listener(&self) {
        // TODO
    }
}
