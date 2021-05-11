//! This is the Raft Distributed Consensus Protocol implemented for Rust.
//! [Raft](http://raftconsensus.github.io/) is described as:
//!
//! > Raft is a consensus algorithm that is designed to be easy to understand. It's equivalent to
//! > Paxos in fault-tolerance and performance. The difference is that it's decomposed into
//! > relatively independent subproblems, and it cleanly addresses all major pieces needed for
//! > practical systems.

mod consensus;
pub mod log;
mod network;
pub mod state_machine;

#[cfg(test)]
mod raft_tests {
    #[test]
    fn test_raft_implementation() {}
}
