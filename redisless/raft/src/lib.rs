//! Raft consensus algorithm implementation.
//!
//! Raft is a consensus algorithm which replicates a strongly-consistent distributed log of entries with arbitrary data
//! amongst a group of peers. It is also fault-tolerant, allowing replication to continue while a majority of peers can
//! still communicate with each other. This crate provides an implementation of the Raft consensus algorithm with some
//! optional features not implemented, such as pre-voting, membership changes, and snapshots.
//!
//! The Raft algorithm is implemented as a state machine driven in a few ways:
//!
//! * When attempting to append a new entry to the distributed log: [`append`](node::Node::append) is called.
//! * When a message is received from a peer: [`receive`](node::Node::receive) is called.
//! * Every time a fixed amount of time has elapsed: [`timer_tick`](node::Node::timer_tick) is called.
//!
//! Each of these functions modifies the internal state and returns [messages](message::SendableMessage) to be sent
//! to peers. Once a log entry is "committed", or guaranteed to be returned at the same index on every functioning peer
//! in the group, it may be retrieved using [`take_committed`](node::Node::take_committed). An append to the log may
//! be cancelled before reaching the committed state, however, which is discussed in more detail in ["Appending entries to the distributed log"].
//!
//! The backing storage for the distributed log must be provided as an implementation of the [`Log`](log::Log)
//! trait, with careful attention to following the trait specification. A trivial in-memory implementation is provided
//! by [`InMemoryLog`](log::mem::InMemoryLog).
//!
//! # Example
//!
//! ```
//! use raft::log::memory::InMemoryLog;
//! use raft::node::{Config, Node};
//! use raft::message::{MessageDestination, SendableMessage};
//! use rand_chacha::ChaChaRng;
//! use rand_core::SeedableRng;
//! use std::collections::VecDeque;
//! use std::str;
//!
//! // Construct 5 Raft peers
//! type NodeId = usize;
//!
//! let mut peers = (0..5).map(|id: NodeId| Node::new(
//!     id,
//!     (0..5).collect(),
//!     InMemoryLog::new_unbounded(),
//!     ChaChaRng::seed_from_u64(id as u64),
//!     Config {
//!         election_timeout_ticks: 10,
//!         heartbeat_interval_ticks: 1,
//!         replication_chunk_size: usize::max_value(),
//!     },
//! )).collect::<Vec<_>>();
//!
//! // Simulate reliably sending messages instantaneously between peers
//! let mut inboxes = vec![VecDeque::new(); peers.len()];
//! let send_message = |src_id: NodeId, sendable: SendableMessage<NodeId>, inboxes: &mut Vec<VecDeque<_>>| {
//!     match sendable.dest {
//!         MessageDestination::Broadcast => {
//!             println!("peer {} -> all: {}", src_id, &sendable.message);
//!             inboxes.iter_mut().for_each(|inbox| inbox.push_back((src_id, sendable.message.clone())))
//!         }
//!         MessageDestination::To(dst_id) => {
//!             println!("peer {} -> peer {}: {}", src_id, dst_id, &sendable.message);
//!             inboxes[dst_id].push_back((src_id, sendable.message));
//!         }
//!     }
//! };
//!
//! // Loop until a log entry is committed on all peers
//! let mut appended = false;
//! let mut peers_committed = vec![false; peers.len()];
//! while !peers_committed.iter().all(|seen| *seen) {
//!     for (peer_id, peer) in peers.iter_mut().enumerate() {
//!         // Tick the timer
//!         let new_messages = peer.timer_tick();
//!         new_messages.for_each(|message| send_message(peer_id, message, &mut inboxes));
//!
//!         // Append a log entry on the leader
//!         if !appended && peer.is_leader() {
//!             if let Ok(new_messages) = peer.append("Hello world!") {
//!                 new_messages.for_each(|message| send_message(peer_id, message, &mut inboxes));
//!                 appended = true;
//!             }
//!         }
//!
//!         // Process message inbox
//!         while let Some((src_id, message)) = inboxes[peer_id].pop_front() {
//!             let new_messages = peer.receive(message, src_id);
//!             new_messages.for_each(|message| send_message(peer_id, message, &mut inboxes));
//!         }
//!
//!         // Check for committed log entries
//!         for log_entry in peer.take_committed() {
//!             if !log_entry.data.is_empty() {
//!                 println!("peer {} saw commit {}", peer_id, str::from_utf8(&log_entry.data).unwrap());
//!                 assert!(!peers_committed[peer_id]);
//!                 peers_committed[peer_id] = true;
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ["Appending entries to the distributed log"]: node::Node#appending-entries-to-the-distributed-log

#![no_std]
#![allow(unused_parens)]
#![warn(missing_docs)]

extern crate alloc;

#[macro_use]
mod macros;

pub mod core;
pub mod log;
pub mod message;
pub mod node;
mod prelude;
