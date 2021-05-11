//! `Consensus` is a state-machine (not to be confused with the `StateMachine` trait) which
//! implements the logic of the Raft Protocol. A `Consensus` receives events from the local
//! `Server`. The set of possible events is specified by the Raft Protocol:
//!
//! ```text
//! Event = AppendEntriesRequest | AppendEntriesResponse
//!       | RequestVoteRequest   | RequestVoteResponse
//!       | ElectionTimeout      | HeartbeatTimeout
//!       | ClientProposal       | ClientQuery
//! ```
//!
//! In response to an event, the `Consensus` may mutate its own state, apply a command to the local
//! `StateMachine`, or return an event to be sent to one or more remote peers or clients.

mod state;
