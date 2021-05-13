//! A simple example with a thread per RaftNode

use std::str;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rand_core::SeedableRng;

use raft::log::memory::InMemoryLog;
use raft::message::{Message, MessageDestination, SendableMessage};
use raft::node::{Config, Node};
use rand_chacha::ChaChaRng;

type NodeId = usize;

const TICK_DURATION: Duration = Duration::from_millis(100);
const RAFT_CONFIG: Config = Config {
    election_timeout_ticks: 10,
    heartbeat_interval_ticks: 1,
    replication_chunk_size: usize::max_value(),
};

#[derive(Clone)]
struct IncomingMessage {
    from: NodeId,
    message: Message,
}

#[derive(Clone)]
struct Network {
    peers_tx: Vec<mpsc::Sender<IncomingMessage>>,
}

fn main() {
    // Construct 5 Raft peers
    let (peers_tx, peers_rx): (Vec<_>, Vec<_>) = (0..5).map(|_| mpsc::channel()).unzip();
    let network = Network { peers_tx };
    let peers = peers_rx
        .into_iter()
        .enumerate()
        .map(|(peer_id, rx): (NodeId, _)| {
            (
                Node::new(
                    peer_id,
                    (0..5).collect(),
                    InMemoryLog::new_unbounded(),
                    ChaChaRng::seed_from_u64(peer_id as u64),
                    RAFT_CONFIG,
                ),
                rx,
            )
        });

    let appended = Arc::new(Mutex::new(false));
    let mut peers_committed = vec![false; peers.len()];
    let (peer_committed_tx, peer_committed_rx) = mpsc::channel();

    for (peer_id, (mut peer, rx)) in peers.enumerate() {
        let appended = Arc::clone(&appended);
        let network = network.clone();
        let peer_committed_tx = peer_committed_tx.clone();
        thread::spawn(move || {
            // Loop until a log entry is committed
            let mut next_tick = Instant::now() + TICK_DURATION;
            loop {
                match rx.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
                    Ok(message) => {
                        // Process incoming message
                        let new_messages = peer.receive(message.message, message.from);
                        new_messages.for_each(|message| network.send(peer_id, message));
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Tick the timer
                        let new_messages = peer.timer_tick();
                        new_messages.for_each(|message| network.send(peer_id, message));
                        next_tick = Instant::now() + TICK_DURATION;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        panic!("peer {} disconnected", peer_id)
                    }
                }

                // Append a log entry on the leader
                let mut appended = appended.lock().unwrap();
                if !*appended && peer.is_leader() {
                    if let Ok(new_messages) = peer.append("Hello world!") {
                        println!("peer {} appending to the log", peer_id);
                        new_messages.for_each(|message| network.send(peer_id, message));
                        *appended = true;
                    }
                }
                drop(appended);

                // Check for committed log entries
                for log_entry in peer.take_committed() {
                    if !log_entry.data.is_empty() {
                        println!(
                            "peer {} saw commit {}",
                            peer_id,
                            str::from_utf8(&log_entry.data).unwrap()
                        );
                        peer_committed_tx.send(peer_id).unwrap();
                    }
                }
            }
        });
    }
    drop((network, peer_committed_tx));

    // Loop until a log entry is committed on all peers
    while !peers_committed.iter().all(|seen| *seen) {
        let peer_id = peer_committed_rx.recv().unwrap();
        assert!(!peers_committed[peer_id]);
        peers_committed[peer_id] = true;
    }
}

impl Network {
    fn send(&self, from: NodeId, sendable: SendableMessage<NodeId>) {
        let message = IncomingMessage {
            from,
            message: sendable.message,
        };
        match sendable.dest {
            MessageDestination::Broadcast => {
                println!("peer {} -> all: {}", from, message.message);
                self.peers_tx
                    .iter()
                    .for_each(|peer_tx| drop(peer_tx.send(message.clone())));
            }
            MessageDestination::To(dst_id) => {
                println!("peer {} -> peer {}: {}", from, dst_id, message.message);
                let _ = self.peers_tx[dst_id].send(message);
            }
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn main() {
        super::main();
    }
}
