//! A simple example from the `raft` crate's crate-level documentation

use std::collections::VecDeque;
use std::str;

use rand_core::SeedableRng;

use raft::log::memory::InMemoryLog;
use raft::message::{MessageDestination, SendableMessage};
use raft::node::{Config, Node};
use rand_chacha::ChaChaRng;

fn main() {
    // Construct 5 Raft peers
    type NodeId = usize;
    let mut peers = (0..5)
        .map(|id: NodeId| {
            Node::new(
                id,
                (0..5).collect(),
                InMemoryLog::new_unbounded(),
                ChaChaRng::seed_from_u64(id as u64),
                Config {
                    election_timeout_ticks: 10,
                    heartbeat_interval_ticks: 1,
                    replication_chunk_size: usize::max_value(),
                },
            )
        })
        .collect::<Vec<_>>();

    // Simulate reliably sending messages instantaneously between peers
    let mut inboxes = vec![VecDeque::new(); peers.len()];
    let send_message =
        |src_id: NodeId, sendable: SendableMessage<NodeId>, inboxes: &mut Vec<VecDeque<_>>| {
            match sendable.dest {
                MessageDestination::Broadcast => {
                    println!("peer {} -> all: {}", src_id, &sendable.message);
                    inboxes
                        .iter_mut()
                        .for_each(|inbox| inbox.push_back((src_id, sendable.message.clone())));
                }
                MessageDestination::To(dst_id) => {
                    println!("peer {} -> peer {}: {}", src_id, dst_id, &sendable.message);
                    inboxes[dst_id].push_back((src_id, sendable.message));
                }
            }
        };

    // Loop until a log entry is committed on all peers
    let mut appended = false;
    let mut peers_committed = vec![false; peers.len()];
    while !peers_committed.iter().all(|seen| *seen) {
        for (peer_id, peer) in peers.iter_mut().enumerate() {
            // Tick the timer
            let new_messages = peer.timer_tick();
            new_messages.for_each(|message| send_message(peer_id, message, &mut inboxes));

            // Append a log entry on the leader
            if !appended && peer.is_leader() {
                if let Ok(new_messages) = peer.append("Hello world!") {
                    println!("peer {} appending to the log", peer_id);
                    new_messages.for_each(|message| send_message(peer_id, message, &mut inboxes));
                    appended = true;
                }
            }

            // Process message inbox
            while let Some((src_id, message)) = inboxes[peer_id].pop_front() {
                let new_messages = peer.receive(message, src_id);
                new_messages.for_each(|message| send_message(peer_id, message, &mut inboxes));
            }

            // Check for committed log entries
            for log_entry in peer.take_committed() {
                if !log_entry.data.is_empty() {
                    println!(
                        "peer {} saw commit {}",
                        peer_id,
                        str::from_utf8(&log_entry.data).unwrap()
                    );
                    assert!(!peers_committed[peer_id]);
                    peers_committed[peer_id] = true;
                }
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
