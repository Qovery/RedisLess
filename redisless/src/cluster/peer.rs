use crate::cluster::node::ClusterNode;
use raft::log::memory::InMemoryLog;
use raft::node::{Config, Node};
use rand::rngs::OsRng;
use std::collections::BTreeSet;
use std::net::SocketAddr;

const CONFIG: Config = Config {
    election_timeout_ticks: 10,
    heartbeat_interval_ticks: 5,
    replication_chunk_size: 65536,
};

type Peers = Vec<Peer>;

#[derive(Debug)]
pub struct Peer {
    id: String,
    socket_addr: SocketAddr,
}

impl Peer {
    pub fn new<T: Into<String>>(id: T, socket_addr: SocketAddr) -> Self {
        Peer {
            id: id.into(),
            socket_addr,
        }
    }

    pub fn into_cluster_node(self, peers: &Peers) -> ClusterNode {
        ClusterNode::new(
            Node::new(
                self.id,
                peers
                    .iter()
                    .map(|peer| peer.id.clone())
                    .collect::<BTreeSet<_>>(),
                InMemoryLog::new_unbounded(),
                OsRng::default(),
                CONFIG,
            ),
            self.socket_addr,
        )
    }
}

#[derive(Debug)]
pub enum PeersDiscovery {
    // peers are provided manually
    Manual(Peers),
    // search peers in the same local network
    Automatic,
}

impl PeersDiscovery {
    // search for peers in the same network
    // 1. scan network
    // 2. for each open TCP socket try to send a discovery payload with the correct Group ID
    // 3. return all peers found.
    fn search_local_peers(&self) -> Peers {
        vec![] // TODO implementation
    }

    pub fn peers(self) -> Peers {
        match self {
            PeersDiscovery::Manual(peers) => peers,
            PeersDiscovery::Automatic => self.search_local_peers(),
        }
    }
}
