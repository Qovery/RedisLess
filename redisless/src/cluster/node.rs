use std::net::SocketAddr;

use rand::rngs::OsRng;

use raft::log::memory::InMemoryLog;
use raft::node::Node;

use crate::cluster::peer::{Peer, PeersDiscovery};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::thread;
use std::time::Duration;

const SEARCH_PEERS_TICK_SECONDS: u64 = 600;
pub const GETINFO_REQUEST: &[u8; 7] = b"getinfo";
pub const GETINFO_RESPONSE: &[u8; 9] = b"redisless";

type RaftNode = Node<InMemoryLog, OsRng, String>;

pub struct ClusterNode {
    #[allow(dead_code)]
    node: RaftNode,

    #[allow(dead_code)]
    listening_socket_addr: SocketAddr,
    
    #[allow(dead_code)]
    peer_receiver: Receiver<Peer>,
    
    listener_started: bool,
    search_peers_started: bool,
}

impl ClusterNode {
    pub fn new(
        node: RaftNode,
        peers_discovery: PeersDiscovery,
        listening_socket_addr: SocketAddr,
    ) -> Self {
        let (tx, rx) = unbounded::<Peer>();

        let mut cn = ClusterNode {
            node,
            listening_socket_addr,
            peer_receiver: rx,
            listener_started: false,
            search_peers_started: false,
        };

        cn.start_search_peers(tx, peers_discovery);

        cn
    }

    /// search for peers every tick
    fn start_search_peers(&mut self, sender: Sender<Peer>, peers_discovery: PeersDiscovery) {
        if self.search_peers_started {
            return;
        }

        let _ = match peers_discovery {
            PeersDiscovery::Manual(_) => return, // in this case - search peers is not useful
            PeersDiscovery::Automatic(_) => {}
        };

        let _ = thread::spawn(move || {
            let tick = Duration::from_secs(SEARCH_PEERS_TICK_SECONDS);
            let sender = sender;
            let peers_discovery = peers_discovery;

            loop {
                thread::sleep(tick);

                // get peers
                for peer in peers_discovery.peers() {
                    let _ = sender.send(peer);
                }
            }
        });

        self.search_peers_started = true;
    }

    // start TCP socket listener to handle incoming message from peers
    pub fn start_listener(&mut self) {
        if self.listener_started {
            return;
        }

        self.listener_started = true;

        // TODO
    }

    // stop TCP socket listener to handle incoming message from peers
    pub fn stop_listener(&mut self) {
        if !self.listener_started {
            return;
        }

        self.listener_started = false;

        // TODO
    }
}
