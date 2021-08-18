use crate::cluster::node::ClusterNode;
use crate::cluster::util::{get_ip_addresses, get_local_network_ip_addresses, scan_ip_range};
use raft::log::memory::InMemoryLog;
use raft::node::{Config, Node};
use rand::rngs::OsRng;
use std::collections::BTreeSet;
use std::net::SocketAddr;

pub const DEFAULT_NODE_LISTENING_PORT: u16 = 8686;

const CONFIG: Config = Config {
    election_timeout_ticks: 10,
    heartbeat_interval_ticks: 5,
    replication_chunk_size: 65536,
};

pub type Peers = Vec<Peer>;

#[derive(Debug, Clone)]
pub struct Peer {
    id: String,
    peers_discovery: PeersDiscovery,
    listening_socket_addr: SocketAddr,
}

impl Peer {
    pub fn new<T: Into<String>>(
        id: T,
        peers_discovery: PeersDiscovery,
        listening_socket_addr: SocketAddr,
    ) -> Self {
        Peer {
            id: id.into(),
            peers_discovery,
            listening_socket_addr,
        }
    }

    pub fn into_cluster_node(self) -> ClusterNode {
        ClusterNode::new(
            Node::new(
                self.id,
                BTreeSet::new(),
                InMemoryLog::new_unbounded(),
                OsRng::default(),
                CONFIG,
            ),
            self.peers_discovery,
            self.listening_socket_addr,
        )
    }
}

type ListeningPort = u16;

#[derive(Debug, Clone)]
pub enum PeersDiscovery {
    // peers are provided manually
    Manual(Peers),
    // search peers in the same local network
    Automatic(ListeningPort),
}

impl PeersDiscovery {
    pub fn peers(&self) -> Peers {
        match self {
            PeersDiscovery::Manual(peers) => peers.clone(),
            PeersDiscovery::Automatic(listening_port) => search_peers(*listening_port),
        }
    }
}

// search for peers in the same network
// 1. scan network
// 2. for each open TCP socket try to send a discovery payload with the correct Group ID
// 3. return all peers found.
fn search_peers(listening_port: u16) -> Peers {
    let local_ip_addresses = get_local_network_ip_addresses(get_ip_addresses());

    // scan those ports - this is an heuristic - that could be improved for sure
    let mut ports: Vec<u16> = (0..2u16).map(|i| listening_port + i).collect();
    if !ports.contains(&DEFAULT_NODE_LISTENING_PORT) {
        ports.insert(0, DEFAULT_NODE_LISTENING_PORT)
    }

    let peers = scan_ip_range(local_ip_addresses, ports);

    peers
        .into_iter()
        .map(|(node_id, socket_addr)| {
            Peer::new(
                node_id,
                PeersDiscovery::Automatic(socket_addr.port()),
                socket_addr,
            )
        })
        .collect()
}
