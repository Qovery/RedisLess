use std::collections::HashMap;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError};
use raft::prelude::*;
use raft::storage::MemStorage;
use raft::{Config, RawNode};

#[derive(Debug, Clone)]
enum Msg {
    Propose {
        id: u8,
        //callback: Box<dyn Fn() + Send>,
    },
    Raft(Message),
}

/// A Node represent a single RedisLess instance within a Cluster.
#[derive(Debug, Clone)]
pub struct Node {
    id: u64,
    socket_addr: SocketAddr,
}

impl Node {
    pub fn new(id: u64, socket_addr: SocketAddr) -> Self {
        Node { id, socket_addr }
    }

    fn raft(&self) -> Result<RawNode<MemStorage>, std::io::Error> {
        let config = Config {
            id: self.id,
            ..Default::default()
        };

        let node = match RawNode::new(&config, MemStorage::default(), vec![]) {
            Ok(raw_node) => raw_node,
            Err(err) => return Err(Error::new(ErrorKind::Other, err.to_string())),
        };

        Ok(node)
    }

    pub fn listen(&self) -> Result<Receiver<Msg>, std::io::Error> {
        let listener = TcpListener::bind(self.socket_addr.to_string())?;

        let (sender, recv) = unbounded::<Msg>();

        let _ = thread::spawn(move || {
            let sender = sender;

            let thread_pool = match rayon::ThreadPoolBuilder::new()
                .thread_name(|_| "raft".to_string())
                .num_threads(4)
                .build()
            {
                Ok(thread_pool) => thread_pool,
                Err(err) => return Err(Error::new(ErrorKind::Other, err.to_string())),
            };

            for tcp_stream in listener.incoming() {
                let tcp_stream = tcp_stream?;
                let sender = sender.clone();
                // process incoming request

                let _ = thread_pool.spawn(move || {
                    let sender = sender.clone();

                    loop {
                        let mut buf_reader = BufReader::new(&tcp_stream);
                        let mut buf = [0; 512];
                        let mut buf_length = 0 as usize;

                        while let Ok(s) = buf_reader.read(&mut buf) {
                            buf_length += s;
                            if s < 512 {
                                break;
                            }
                        }

                        // TODO convert bytes to Message payload
                        // TODO and use sender.send(msg)
                    }
                });
            }

            Ok(())
        });

        Ok(recv)
    }

    pub fn send(&self, message: &Message) {
        // TODO
    }
}

/// A Cluster represent nodes that are connected altogether and work as a single unit.
pub struct Cluster {
    current_node: Node,
    peer_nodes: Arc<Vec<Node>>,
}

impl Cluster {
    pub fn new(current_node: Node, peer_nodes: Vec<Node>) -> Self {
        Cluster {
            current_node,
            peer_nodes: Arc::new(peer_nodes),
        }
    }

    pub fn init(&self) -> Result<(), std::io::Error> {
        let receiver = self.current_node.listen()?;
        let raft = self.current_node.raft()?;
        let peer_nodes = self.peer_nodes.clone();

        let _ = thread::spawn(move || {
            let mut now = Instant::now();
            let timeout = Duration::from_millis(100);
            let mut remaining_timeout = timeout;
            let mut raft = raft;
            let peer_nodes = peer_nodes;

            loop {
                match receiver.recv_timeout(timeout) {
                    Ok(Msg::Propose { id }) => {
                        let _ = raft.propose(vec![], vec![id]); // TODO catch errors
                    }
                    Ok(Msg::Raft(msg)) => {
                        let _ = raft.step(msg); // TODO catch errors
                    }
                    Err(RecvTimeoutError::Timeout) => (),
                    Err(RecvTimeoutError::Disconnected) => break,
                }

                let elapsed = now.elapsed();
                if elapsed >= timeout {
                    remaining_timeout = timeout;
                    // We drive Raft every 100ms.
                    raft.tick();
                } else {
                    remaining_timeout -= elapsed;
                }

                on_ready(&mut raft, &peer_nodes);
            }
        });

        Ok(())
    }
}

fn on_ready(raft: &mut RawNode<MemStorage>, peer_nodes: &Vec<Node>) {
    if !raft.has_ready() {
        return;
    }

    let store = raft.raft.raft_log.store.clone();

    // Get the `Ready` with `RawNode::ready` interface.
    let mut ready = raft.ready();

    let handle_messages = |msgs: &Vec<Message>| {
        for msg in msgs {
            // Send messages to other peers.
            for node in peer_nodes {
                node.send(&msg);
            }
        }
    };

    // Send out the messages come from the node.
    handle_messages(&ready.messages);

    if !raft::is_empty_snap(ready.snapshot()) {
        // This is a snapshot, we need to apply the snapshot at first.
        store.wl().apply_snapshot(ready.snapshot().clone()).unwrap();
    }

    let mut handle_committed_entries = |committed_entries: Option<&Vec<Entry>>| {
        let committed_entries = match committed_entries {
            Some(committed_entries) => committed_entries,
            None => return,
        };

        let mut last_apply_index = 0;
        for entry in committed_entries {
            last_apply_index = handle_entry(entry);
        }
    };

    handle_committed_entries(ready.committed_entries.as_ref());

    if !ready.entries().is_empty() {
        // Append entries to the Raft log.
        store.wl().append(ready.entries()).unwrap();
    }

    if let Some(hs) = ready.hs() {
        // Raft HardState changed, and we need to persist it.
        store.wl().set_hardstate(hs.clone());
    }

    if let Some(committed_entries) = ready.committed_entries.take() {
        let mut last_apply_index = 0;
        for entry in committed_entries.iter() {
            last_apply_index = handle_entry(entry);
        }
    }

    raft.advance(ready);
}

fn handle_entry(entry: &Entry) -> u64 {
    // Mostly, you need to save the last apply index to resume applying
    // after restart. Here we just ignore this because we use a Memory storage.
    let last_apply_index = entry.get_index();

    if entry.get_data().is_empty() {
        // Emtpy entry, when the peer becomes Leader it will send an empty entry.
        return last_apply_index;
    }

    match entry.get_entry_type() {
        EntryType::EntryNormal => {}     // TODO handle normal entry
        EntryType::EntryConfChange => {} // TODO handle conf change
    };

    last_apply_index
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::thread;

    use rand::{thread_rng, RngCore};

    use crate::cluster::{Cluster, Node};

    #[test]
    fn start_and_stop_cluster() {
        let mut rng = thread_rng();

        let n1 = Node::new(
            rng.next_u64(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5555),
        );

        let n2 = Node::new(
            rng.next_u64(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5556),
        );

        let n3 = Node::new(
            rng.next_u64(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5557),
        );

        let _ = n2.listen();
        let _ = n3.listen();

        let cluster = Cluster::new(n1, vec![n2, n3]);

        let _ = cluster.init();
    }
}
