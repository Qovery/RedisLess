//! Unstable, low-level API for the complete state of a Raft node.

use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt;
use core::iter;

use bytes::Bytes;
use log::{debug, error, info, warn};
use rand_core::RngCore;

use crate::log::{CommittedIter, Log, LogState};
use crate::message::*;
use crate::node::{AppendError, Config};
use crate::prelude::*;

use self::LeadershipState::*;

/// The state of Raft log replication from a Raft node to one of its peers.
pub struct ReplicationState {
    // \* The next entry to send to each follower.
    // VARIABLE nextIndex
    /// The index of the next log entry to be sent to this peer.
    pub next_idx: LogIndex,

    // \* The latest entry that each follower has acknowledged is the same as the
    // \* leader's. This is used to calculate commitIndex on the leader.
    // VARIABLE matchIndex
    /// The index of the last log entry on this peer to up which the peer's log is known to match this node's log.
    pub match_idx: LogIndex,

    /// The index of the last log entry sent to this peer but which has not yet been acknowledged by the peer.
    pub inflight: Option<LogIndex>,

    /// Whether this node is currently probing to discover the correct [`match_idx`][Self::match_idx] for this peer.
    pub send_probe: bool,

    /// Whether a heartbeat "ping" message is due to be sent to this peer.
    send_heartbeat: bool,
}

// \* Server states.
// CONSTANTS Follower, Candidate, Leader
enum LeadershipState<NodeId> {
    Follower(FollowerState<NodeId>),
    Candidate(CandidateState<NodeId>),
    Leader(LeaderState<NodeId>),
}

struct FollowerState<NodeId> {
    leader: Option<NodeId>,

    election_ticks: u32,
    random_election_ticks: u32,
}

struct CandidateState<NodeId> {
    // \* The latest entry that each follower has acknowledged is the same as the
    // \* leader's. This is used to calculate commitIndex on the leader.
    // VARIABLE votesGranted
    votes_granted: BTreeSet<NodeId>,

    election_ticks: u32,
}

struct LeaderState<NodeId> {
    followers: BTreeMap<NodeId, ReplicationState>,

    heartbeat_ticks: u32,
}

/// The complete state of a Raft node.
pub struct State<L, Random, NodeId> {
    node_id: NodeId,
    peers: BTreeSet<NodeId>,
    random: Random,
    config: Config,

    // \* The server's term number.
    // VARIABLE currentTerm
    current_term: TermId,

    // \* The candidate the server voted for in its current term, or
    // \* Nil if it hasn't voted for any.
    // VARIABLE votedFor
    voted_for: Option<NodeId>,

    // \* The server's state (Follower, Candidate, or Leader).
    // VARIABLE state
    leadership: LeadershipState<NodeId>,

    // \* A Sequence of log entries. The index into this sequence is the index of the
    // \* log entry. Unfortunately, the Sequence module defines Head(s) as the entry
    // \* with index 1, so be careful not to use that!
    // VARIABLE log
    // \* The index of the latest entry in the log the state machine may apply.
    // VARIABLE commitIndex
    log: LogState<L>,
}

#[allow(missing_docs)]
impl<L, Random, NodeId> State<L, Random, NodeId>
where
    L: Log,
    Random: RngCore,
    NodeId: Ord + Clone + fmt::Display,
{
    pub fn new(
        node_id: NodeId,
        mut peers: BTreeSet<NodeId>,
        log: L,
        mut random: Random,
        config: Config,
    ) -> Self {
        peers.remove(&node_id);
        let random_election_ticks =
            random_election_timeout(&mut random, config.election_timeout_ticks);
        Self {
            node_id,
            peers,
            random,
            config,
            log: LogState::new(log),
            current_term: Default::default(),
            voted_for: Default::default(),
            leadership: Follower(FollowerState {
                leader: None,
                election_ticks: random_election_ticks,
                random_election_ticks,
            }),
        }
    }

    pub fn commit_idx(&self) -> &LogIndex {
        &self.log.commit_idx
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn is_leader(&self) -> bool {
        if let Leader(_) = &self.leadership {
            true
        } else {
            false
        }
    }

    pub fn leader(&self) -> (Option<&NodeId>, &TermId) {
        let leader = match &self.leadership {
            Follower(follower_state) => follower_state.leader.as_ref(),
            Candidate(_) => None,
            Leader(_) => Some(&self.node_id),
        };
        (leader, &self.current_term)
    }

    pub fn log(&self) -> &L {
        self.log.log()
    }

    pub fn log_mut(&mut self) -> &mut L {
        self.log.log_mut()
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub fn peers(&self) -> &BTreeSet<NodeId> {
        &self.peers
    }

    pub fn replication_state(&self, peer_node_id: &NodeId) -> Option<&ReplicationState> {
        if let LeadershipState::Leader(leader_state) = &self.leadership {
            leader_state.followers.get(peer_node_id)
        } else {
            None
        }
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;

        match &mut self.leadership {
            Follower(FollowerState {
                election_ticks,
                random_election_ticks,
                ..
            }) => {
                if *random_election_ticks > self.config.election_timeout_ticks.saturating_mul(2) {
                    *random_election_ticks = random_election_timeout(
                        &mut self.random,
                        self.config.election_timeout_ticks,
                    );
                }
                if election_ticks > random_election_ticks {
                    *election_ticks = *random_election_ticks;
                }
            }
            Candidate(CandidateState { election_ticks, .. }) => {
                if *election_ticks > self.config.election_timeout_ticks.saturating_mul(2) {
                    *election_ticks = random_election_timeout(
                        &mut self.random,
                        self.config.election_timeout_ticks,
                    );
                }
            }
            Leader(LeaderState {
                heartbeat_ticks, ..
            }) => {
                if *heartbeat_ticks > self.config.heartbeat_interval_ticks {
                    *heartbeat_ticks = self.config.heartbeat_interval_ticks;
                }
            }
        }
    }

    pub fn take_committed(&mut self) -> CommittedIter<'_, L> {
        self.log.take_committed()
    }

    pub fn timer_tick(&mut self) -> Option<SendableMessage<NodeId>> {
        match &mut self.leadership {
            Follower(FollowerState { election_ticks, .. })
            | Candidate(CandidateState { election_ticks, .. }) => {
                match election_ticks.saturating_sub(1) {
                    0 => {
                        info!("election timeout at {}", &self.current_term);
                        self.timeout()
                    }
                    new_election_ticks => {
                        *election_ticks = new_election_ticks;
                        None
                    }
                }
            }
            Leader(leader_state) => {
                match leader_state.heartbeat_ticks.saturating_sub(1) {
                    0 => {
                        leader_state.heartbeat_ticks = self.config.heartbeat_interval_ticks;
                        debug!("sending heartbeat");
                        for replication in leader_state.followers.values_mut() {
                            replication.send_heartbeat = true;
                        }
                    }
                    new_heartbeat_ticks => {
                        leader_state.heartbeat_ticks = new_heartbeat_ticks;
                    }
                }
                None
            }
        }
    }

    pub fn reset_peer(&mut self, peer_node_id: NodeId) -> Option<SendableMessage<NodeId>> {
        match &mut self.leadership {
            Follower(_) => None,
            Candidate(_) => {
                if self.peers.contains(&peer_node_id) {
                    let vote_request = self.request_vote();
                    let from = peer_node_id;
                    vote_request.map(|message| SendableMessage {
                        message,
                        dest: MessageDestination::To(from),
                    })
                } else {
                    None
                }
            }
            Leader(leader_state) => {
                if let Some(replication) = leader_state.followers.get_mut(&peer_node_id) {
                    info!("resetting follower state {}", &peer_node_id);
                    replication.next_idx = self.log.last_index() + 1;
                    replication.send_probe = true;
                    replication.send_heartbeat = true;
                    replication.inflight = None;
                }
                None
            }
        }
    }

    //
    // -- raft TLA+ parallel code --
    // the code below is so similar to Raft's TLA+ code that the TLA+ is provided
    // in the right-hand column for sections which correspond almost exactly. code
    // is provided in the same order as the TLA+ so that the reader can follow.
    //

    //
    // \* Define state transitions
    //

    // \* Server i times out and starts a new election.
    pub fn timeout(&mut self) -> Option<SendableMessage<NodeId>> {
        // Timeout(i) ==
        match &self.leadership {
            Follower(_) | Candidate(_) => {
                // /\ state[i] \in {Follower, Candidate}
                self.current_term += 1; // /\ currentTerm' = [currentTerm EXCEPT ![i] = currentTerm[i] + 1]
                                        // \* Most implementations would probably just set the local vote
                                        // \* atomically, but messaging localhost for it is weaker.
                self.voted_for = Some(self.node_id.clone()); // /\ votedFor' = [votedFor EXCEPT ![i] = Nil]
                let votes_granted = iter::once(self.node_id.clone()).collect(); // /\ votesGranted'   = [votesGranted EXCEPT ![i] = {}]
                self.leadership = Candidate(CandidateState {
                    // /\ state' = [state EXCEPT ![i] = Candidate]
                    votes_granted,
                    election_ticks: self.random_election_timeout(),
                });

                info!("became candidate at {}", self.current_term);
                self.become_leader();
                self.advance_commit_idx();
                self.request_vote().map(|message| SendableMessage {
                    message,
                    dest: MessageDestination::Broadcast,
                })
            }
            Leader(_) => None,
        }
    }

    // \* Candidate i sends j a RequestVote request.
    fn request_vote(&self) -> Option<Message> {
        // RequestVote(i,j) ==
        match self.leadership {
            Candidate { .. } => {
                // /\ state[i] = Candidate
                let vote_request_msg = Message {
                    // /\ Send([
                    term: self.current_term, //          mterm         |-> currentTerm[i],
                    rpc: Some(Rpc::VoteRequest(VoteRequest {
                        //          mtype         |-> RequestVoteRequest,
                        last_log_term: self.log.last_term(), //          mlastLogTerm  |-> LastTerm(log[i]),
                        last_log_idx: self.log.last_index(), //          mlastLogIndex |-> Len(log[i]),
                    })),
                };
                Some(vote_request_msg)
            }
            _ => None,
        }
    }

    // \* Leader i sends j an AppendEntries request containing up to 1 entry.
    // \* While implementations may want to send more than 1 at a time, this spec uses
    // \* just 1 because it minimizes atomic regions without loss of generality.
    pub fn append_entries(&mut self, to_node_id: NodeId) -> Option<SendableMessage<NodeId>> {
        // AppendEntries(i, j) ==
        if let Leader(leader_state) = &mut self.leadership {
            // /\ state[i] = Leader
            let replication = match leader_state.followers.get_mut(&to_node_id) {
                // /\ i /= j
                Some(replication) => replication,
                None => return None,
            };
            let last_log_idx = self.log.last_index();
            let next_idx = replication.next_idx;
            let send_entries = (last_log_idx >= next_idx && !replication.send_probe);
            if !send_entries && !replication.send_heartbeat {
                return None;
            }
            if replication.inflight.is_some() {
                return None;
            }
            let prev_log_idx = next_idx - 1; // /\ LET prevLogIndex == nextIndex[i][j] - 1
            let maybe_prev_log_term = if prev_log_idx != Default::default() {
                //        prevLogTerm == IF prevLogIndex > 0 THEN
                self.log.get_term(prev_log_idx) //                           log[i][prevLogIndex].term
            } else {
                //                       ELSE
                Some(Default::default()) //                           0
            };

            let prev_log_term = match maybe_prev_log_term {
                Some(prev_log_term) => prev_log_term,
                None => {
                    error!("missing log {} to send to {}!", &prev_log_idx, &to_node_id);
                    return None;
                }
            };

            let mut entries: Vec<LogEntry> = Vec::new();
            let last_entry: LogIndex;
            if send_entries {
                //        \* Send up to 1 entry, constrained by the end of the log.
                let mut entries_size = 0usize;
                let max_entries_size = self.config.replication_chunk_size;
                let entry_log_idxs = (0..)
                    .map(|idx| next_idx + idx)
                    .take_while(|log_idx| *log_idx <= last_log_idx);
                for entry_log_idx in entry_log_idxs {
                    //        entries == SubSeq(log[i], nextIndex[i][j], lastEntry)
                    let append_log_entry = if let Some(log_entry) = self.log.get(entry_log_idx) {
                        let first_entry = entries_size == 0;
                        if !first_entry && entries_size == max_entries_size {
                            None
                        } else {
                            entries_size =
                                entries_size.saturating_add(self.log.entry_len(&log_entry));
                            if first_entry || entries_size <= max_entries_size {
                                Some(log_entry)
                            } else {
                                None
                            }
                        }
                    } else {
                        error!(
                            "error fetching raft log {} to send to {}!",
                            &entry_log_idx, &to_node_id
                        );
                        None
                    };
                    if let Some(log_entry) = append_log_entry {
                        entries.push(log_entry);
                    } else {
                        break;
                    }
                }
                last_entry = prev_log_idx + (entries.len() as u64); //        lastEntry == Min({Len(log[i]), nextIndex[i][j]})
            } else {
                last_entry = prev_log_idx;
            }
            let append_request_msg = Message {
                //    IN Send([
                term: self.current_term, //             mterm          |-> currentTerm[i],
                rpc: Some(Rpc::AppendRequest(AppendRequest {
                    //             mtype          |-> AppendEntriesRequest,
                    prev_log_idx,  //             mprevLogIndex  |-> prevLogIndex,
                    prev_log_term, //             mprevLogTerm   |-> prevLogTerm,
                    entries,       //             mentries       |-> entries,
                    leader_commit: self.log.commit_idx.min(last_entry), //             mcommitIndex   |-> Min({commitIndex[i], lastEntry}),
                })),
            };
            replication.send_heartbeat = false;
            replication.inflight = Some(last_entry);
            Some(SendableMessage {
                message: append_request_msg,
                dest: MessageDestination::To(to_node_id),
            })
        } else {
            None
        }
    }

    // \* Candidate i transitions to leader.
    fn become_leader(&mut self) {
        // BecomeLeader(i) ==
        if let Candidate(candidate_state) = &self.leadership {
            // /\ state[i] = Candidate
            if candidate_state.votes_granted.len() >= self.quorum_size() {
                // /\ votesGranted[i] \in Quorum
                info!("became leader at {}", &self.current_term);
                self.leadership = Leader(LeaderState {
                    // /\ state'      = [state EXCEPT ![i] = Leader]
                    followers: (self.peers.iter().cloned())
                        .map(|id| {
                            (
                                id,
                                ReplicationState {
                                    next_idx: self.log.last_index() + 1, // /\ nextIndex'  = [nextIndex EXCEPT ![i] = [j \in Server |-> Len(log[i]) + 1]]
                                    match_idx: Default::default(), // /\ matchIndex' = [matchIndex EXCEPT ![i] = [j \in Server |-> 0]]
                                    inflight: Default::default(),
                                    send_probe: Default::default(),
                                    send_heartbeat: Default::default(),
                                },
                            )
                        })
                        .collect(),
                    heartbeat_ticks: 0,
                });
                // append a noop in the new term to commit entries from past terms (Raft Section 5.4.2)
                let _ignore = self.client_request(Default::default());
            }
        }
    }

    // \* Leader i receives a client request to add v to the log.
    pub fn client_request(&mut self, data: Bytes) -> Result<(), AppendError<L::Error>> {
        // ClientRequest(i, v) ==
        let entry = LogEntry {
            term: self.current_term, // /\ LET entry == [term  |-> currentTerm[i],
            data,                    //                  value |-> v]
        };

        if let Leader(_) = &self.leadership {
            // /\ state[i] = Leader
            self.log.append(entry).map_err(AppendError::LogErr)?; //        newLog == Append(log[i], entry)
            self.advance_commit_idx();
            Ok(()) //    IN  log' = [log EXCEPT ![i] = newLog]
        } else {
            Err(AppendError::Cancelled { data: entry.data })
        }
    }

    // \* Leader i advances its commitIndex.
    // \* This is done as a separate step from handling AppendEntries responses,
    // \* in part to minimize atomic regions, and in part so that leaders of
    // \* single-server clusters are able to mark entries committed.
    fn advance_commit_idx(&mut self) {
        // AdvanceCommitIndex(i) ==
        if let Leader(leader_state) = &self.leadership {
            // /\ state[i] = Leader
            let mut match_idxs: Vec<_> =                                        // /\ LET \* The set of servers that agree up through index.
                (leader_state.followers.values())
                    .map(|follower| follower.match_idx)
                    .chain(iter::once(self.log.last_index()))
                    .collect();
            match_idxs.sort_unstable(); //        Agree(index) == {i} \cup {k \in Server : matchIndex[i][k] >= index}
            let agree_idxs = (match_idxs.into_iter()) //        \* The maximum indexes for which a quorum agrees
                .rev()
                .skip(self.quorum_size() - 1); //        agreeIndexes == {index \in 1..Len(log[i]) : Agree(index) \in Quorum}
            let commit_idx = match agree_idxs.max() {
                //        \* New value for commitIndex'[i]
                Some(agree_idx) => {
                    //        newCommitIndex == IF /\ agreeIndexes /= {}
                    if self.log.get_term(agree_idx) == Some(self.current_term) {
                        //                             /\ log[i][Max(agreeIndexes)].term = currentTerm[i]
                        self.log.commit_idx.max(agree_idx) //                          THEN Max(agreeIndexes)
                    } else {
                        self.log.commit_idx //                          ELSE commitIndex[i]
                    }
                }
                None => self.log.commit_idx,
            };
            if commit_idx != self.log.commit_idx {
                debug!(
                    "committed transactions from {} to {}",
                    &self.log.commit_idx, &commit_idx
                );
            }
            self.log.commit_idx = commit_idx; //    IN commitIndex' = [commitIndex EXCEPT ![i] = newCommitIndex]
        }
    }

    //
    // \* Message handlers
    // \* i = recipient, j = sender, m = message
    //

    // \* Server i receives a RequestVote request from server j with
    // \* m.mterm <= currentTerm[i].
    fn handle_vote_request(
        &mut self,
        msg_term: TermId,
        msg: VoteRequest,
        from: NodeId,
    ) -> Option<SendableMessage<NodeId>> {
        // HandleRequestVoteRequest(i, j, m) ==
        let last_log_idx = self.log.last_index();
        let last_log_term = self.log.last_term();
        let log_ok =                                                            // LET logOk ==
            (msg.last_log_term > last_log_term) ||                             //     \/ m.mlastLogTerm > LastTerm(log[i])
                (msg.last_log_term == last_log_term &&                              //     \/ /\ m.mlastLogTerm = LastTerm(log[i])
                    msg.last_log_idx >= last_log_idx); //        /\ m.mlastLogIndex >= Len(log[i])
        let grant =                                                             // LET grant ==
            msg_term == self.current_term &&                                    //     /\ m.mterm = currentTerm[i]
                log_ok &&                                                           //     /\ logOk
                self.voted_for.as_ref().map(|vote| &from == vote).unwrap_or(true); //     /\ votedFor[i] \in {Nil, j}
        assert!(msg_term <= self.current_term); // IN /\ m.mterm <= currentTerm[i]
        if grant {
            self.voted_for = Some(from.clone()); //    /\ \/ grant  /\ votedFor' = [votedFor EXCEPT ![i] = j]
        } //       \/ ~grant /\ UNCHANGED votedFor

        if grant {
            info!(
                "granted vote at {} with {} at {} for node {} with {} at {}",
                &self.current_term,
                &last_log_idx,
                &last_log_term,
                &from,
                &msg.last_log_idx,
                &msg.last_log_term
            );
            match &mut self.leadership {
                Follower(FollowerState {
                    election_ticks,
                    random_election_ticks,
                    ..
                }) => *election_ticks = *random_election_ticks,
                Candidate(_) | Leader(_) => (),
            }
        } else if msg_term != self.current_term {
            info!(
                "ignored message with {} < current {}: {}",
                &msg_term, &self.current_term, &msg
            );
        } else if let Some(vote) = &self.voted_for {
            info!(
                "rejected vote at {} for node {} as already voted for {}",
                &self.current_term, &from, vote
            );
        } else {
            info!(
                "rejected vote at {} with {} at {} for node {} with {} at {}",
                &self.current_term,
                &last_log_idx,
                &last_log_term,
                &from,
                &msg.last_log_idx,
                &msg.last_log_term
            );
        }

        let message = Message {
            // /\ Reply([
            term: self.current_term, //           mterm        |-> currentTerm[i],
            rpc: Some(Rpc::VoteResponse(VoteResponse {
                //           mtype        |-> RequestVoteResponse,
                vote_granted: grant, //           mvoteGranted |-> grant,
            })),
        };
        Some(SendableMessage {
            message,
            dest: MessageDestination::To(from),
        })
    }

    // \* Server i receives a RequestVote response from server j with
    // \* m.mterm = currentTerm[i].
    fn handle_vote_response(
        &mut self,
        msg_term: TermId,
        msg: VoteResponse,
        from: NodeId,
    ) -> Option<SendableMessage<NodeId>> {
        // HandleRequestVoteResponse(i, j, m) ==
        assert!(msg_term == self.current_term); // /\ m.mterm = currentTerm[i]
        if let Candidate(candidate_state) = &mut self.leadership {
            if msg.vote_granted {
                // /\ \/ /\ m.mvoteGranted
                info!(
                    "received vote granted from {} at {}",
                    &from, &self.current_term
                );
                candidate_state.votes_granted.insert(from); //       /\ votesGranted' = [votesGranted EXCEPT ![i] = votesGranted[i] \cup {j}]
            } else {
                //    \/ /\ ~m.mvoteGranted /\ UNCHANGED <<votesGranted, voterLog>>
                info!(
                    "received vote rejected from {} at {}",
                    &from, &self.current_term
                );
            }
        }
        None
    }

    // \* Server i receives an AppendEntries request from server j with
    // \* m.mterm <= currentTerm[i]. This just handles m.entries of length 0 or 1, but
    // \* implementations could safely accept more by treating them the same as
    // \* multiple independent requests of 1 entry.
    fn handle_append_request(
        &mut self,
        msg_term: TermId,
        msg: AppendRequest,
        from: NodeId,
    ) -> Option<SendableMessage<NodeId>> {
        // HandleAppendEntriesRequest(i, j, m) ==
        let prev_log_idx = msg.prev_log_idx;
        let msg_prev_log_term = msg.prev_log_term;
        let our_prev_log_term = self.log.get_term(prev_log_idx);
        let log_ok = prev_log_idx == Default::default() ||                               // LET logOk == \/ m.mprevLogIndex = 0
            Some(msg_prev_log_term) == our_prev_log_term; //              \/ /\ m.mprevLogIndex > 0 /\ m.mprevLogIndex <= Len(log[i]) /\ m.mprevLogTerm = log[i][m.mprevLogIndex].term
        assert!(msg_term <= self.current_term); // IN /\ m.mterm <= currentTerm[i]
                                                //    /\ \/ \* return to follower state
        if msg_term == self.current_term {
            //          /\ m.mterm = currentTerm[i]
            match &mut self.leadership {
                Candidate(_) => {
                    //          /\ state[i] = Candidate
                    let random_election_ticks = self.random_election_timeout();
                    self.leadership = Follower(FollowerState {
                        //          /\ state' = [state EXCEPT ![i] = Follower]
                        leader: Some(from.clone()),
                        election_ticks: random_election_ticks,
                        random_election_ticks,
                    });
                    info!("became follower at {} of {}", &self.current_term, &from);
                }
                Follower(follower_state) => {
                    if follower_state.leader.is_none() {
                        info!("became follower at {} of {}", &self.current_term, &from);
                    }
                    follower_state.leader = Some(from.clone());
                    follower_state.election_ticks = follower_state.random_election_ticks;
                }
                Leader { .. } => {
                    panic!(
                        "received append request as leader at {} from {}",
                        &self.current_term, &from
                    );
                }
            }
        }
        //       \/ /\ \* reject request
        if msg_term < self.current_term ||                                     //             \/ m.mterm < currentTerm[i]
            (assert_true!(msg_term == self.current_term) &&                     //             \/ /\ m.mterm = currentTerm[i]
                assert_match!(Follower(_) = &self.leadership) &&                   //                /\ state[i] = Follower
                !log_ok)
        //                /\ \lnot logOk
        {
            if msg_term < self.current_term {
                info!(
                    "ignored message with {} < current {}: {}",
                    &msg_term, &self.current_term, &msg
                );
            } else if let Some(our_prev_log_term) = our_prev_log_term {
                warn!(
                    "rejected append from {} with {} at {}, we have {}",
                    &from, &prev_log_idx, msg_prev_log_term, &our_prev_log_term
                );
            } else {
                info!(
                    "rejected append from {} with {}, we are behind at {}",
                    &from,
                    &prev_log_idx,
                    self.log.last_index()
                );
            }

            let message = Message {
                //                /\ Reply([
                term: self.current_term, //                          mterm           |-> currentTerm[i],
                rpc: Some(Rpc::AppendResponse(AppendResponse {
                    //                          mtype           |-> AppendEntriesResponse,
                    success: false, //                          msuccess        |-> FALSE,
                    match_idx: self.log.prev_index(), //                          mmatchIndex     |-> 0,
                    last_log_idx: self.log.last_index(),
                })),
            };
            Some(SendableMessage {
                message,
                dest: MessageDestination::To(from),
            })
        } else {
            //       \/ \* accept request
            assert!(msg_term == self.current_term);
            //          /\ m.mterm = currentTerm[i]
            assert_match!(Follower(_) = &self.leadership); //          /\ state[i] = Follower
            assert!(log_ok); //          /\ logOk
                             // ... and the TLA+ that follows doesn't correspond to procedural code well
                             // find point of log conflict
            let msg_last_log_idx = prev_log_idx + (msg.entries.len() as u64);
            let msg_entries_iter = (1..).map(|idx| prev_log_idx + idx).zip(msg.entries);
            let mut last_processed_idx = prev_log_idx;
            for (msg_entry_log_idx, msg_entry) in msg_entries_iter {
                if msg_entry_log_idx == self.log.last_index() + 1 {
                    match self.log.append(msg_entry) {
                        Ok(()) => (),
                        Err(_) => break,
                    }
                } else if let Some(our_entry_log_term) = self.log.get_term(msg_entry_log_idx) {
                    if our_entry_log_term != msg_entry.term {
                        assert!(msg_entry_log_idx > self.log.commit_idx);
                        match self.log.cancel_from(msg_entry_log_idx) {
                            Ok(cancelled_len) => info!(
                                "cancelled {} transactions from {}",
                                cancelled_len, &msg_entry_log_idx
                            ),
                            Err(_) => break,
                        }
                        match self.log.append(msg_entry) {
                            Ok(()) => (),
                            Err(_) => break,
                        }
                    }
                } else {
                    error!(
                        "failed to fetch log index {} to find conflicts for append!",
                        &msg_entry_log_idx
                    );
                    break;
                }
                last_processed_idx = msg_entry_log_idx;
            }

            // update commit index from leader
            let leader_commit = msg.leader_commit.min(last_processed_idx);
            if leader_commit > self.log.commit_idx {
                debug!(
                    "committed transactions from {} to {}",
                    &self.log.commit_idx, &leader_commit
                );

                self.log.commit_idx = leader_commit; // /\ commitIndex' = [commitIndex EXCEPT ![i] = m.mcommitIndex]
            }

            let message = Message {
                // /\ Reply([
                term: self.current_term, //           mterm           |-> currentTerm[i],
                rpc: Some(Rpc::AppendResponse(AppendResponse {
                    //           mtype           |-> AppendEntriesResponse,
                    success: true, //           msuccess        |-> TRUE,
                    match_idx: msg_last_log_idx.min(self.log.last_index()), //        mmatchIndex     |-> m.mprevLogIndex + Len(m.mentries),
                    last_log_idx: self.log.last_index(),
                })),
            };
            Some(SendableMessage {
                message,
                dest: MessageDestination::To(from),
            })
        }
    }

    // \* Server i receives an AppendEntries response from server j with
    // \* m.mterm = currentTerm[i].
    fn handle_append_response(
        &mut self,
        msg_term: TermId,
        msg: AppendResponse,
        from: NodeId,
    ) -> Option<SendableMessage<NodeId>> {
        // HandleAppendEntriesResponse(i, j, m) ==
        assert!(msg_term == self.current_term); // /\ m.mterm = currentTerm[i]
        if let Leader(leader_state) = &mut self.leadership {
            if let Some(replication) = leader_state.followers.get_mut(&from) {
                if msg.success {
                    // /\ \/ /\ m.msuccess \* successful
                    if Some(msg.match_idx) >= replication.inflight {
                        replication.inflight = None;
                    }
                    if msg.match_idx + 1 > replication.next_idx {
                        replication.next_idx = msg.match_idx + 1; //       /\ nextIndex'  = [nextIndex  EXCEPT ![i][j] = m.mmatchIndex + 1]
                    }
                    if msg.match_idx > replication.match_idx {
                        replication.match_idx = msg.match_idx; //       /\ matchIndex' = [matchIndex EXCEPT ![i][j] = m.mmatchIndex]
                    }
                    replication.send_probe = false;
                } else {
                    //    \/ /\ \lnot m.msuccess \* not successful
                    if !replication.send_probe {
                        info!(
                            "received append rejection at {} from {} having {}",
                            &replication.next_idx, &from, &msg.last_log_idx
                        );
                    } else {
                        verbose!(
                            "received append rejection at {} from {} having {}",
                            &replication.next_idx,
                            &from,
                            &msg.last_log_idx
                        );
                    }
                    replication.next_idx = ((replication.next_idx - 1) //       /\ nextIndex' = [nextIndex EXCEPT ![i][j] = Max({nextIndex[i][j] - 1, 1})]
                        .min(msg.last_log_idx + 1)
                        .max(msg.match_idx + 1));
                    replication.send_probe = true;
                    replication.inflight = None;

                    let mut chunk_size_remaining = self.config.replication_chunk_size;
                    while let Some(next_idx) = replication.next_idx.checked_sub(1) {
                        if next_idx <= msg.match_idx {
                            break;
                        }
                        let entry_len = match self.log.get_len(replication.next_idx) {
                            Some(entry_len) => entry_len,
                            None => break,
                        };
                        chunk_size_remaining = match chunk_size_remaining.checked_sub(entry_len) {
                            Some(new_chunk_size_remaining) => new_chunk_size_remaining,
                            None => break,
                        };
                        replication.next_idx = next_idx;
                    }
                }
            }
        }
        None
    }

    // \* Any RPC with a newer term causes the recipient to advance its term first.
    fn update_term(&mut self, from: &NodeId, msg: &Message) {
        // UpdateTerm(i, j, m) ==
        if msg.term > self.current_term {
            // /\ m.mterm > currentTerm[i]
            info!(
                "became follower at {} (from {}) due to message from {}: {}",
                &msg.term, &self.current_term, from, &msg
            );
            let random_election_ticks = self.random_election_timeout();

            let election_ticks = match &self.leadership {
                Follower(FollowerState { election_ticks, .. })
                | Candidate(CandidateState { election_ticks, .. }) => *election_ticks,
                Leader(_) => random_election_ticks,
            };
            self.current_term = msg.term; // /\ currentTerm'    = [currentTerm EXCEPT ![i] = m.mterm]
            self.leadership = Follower(FollowerState {
                // /\ state'          = [state       EXCEPT ![i] = Follower]
                leader: None,
                election_ticks,
                random_election_ticks,
            });
            self.voted_for = Default::default(); // /\ votedFor'       = [votedFor    EXCEPT ![i] = Nil]
        }
    }

    // \* Responses with stale terms are ignored.
    fn drop_stale_response<T>(&self, msg_term: TermId, msg: T) -> Result<(), T>
    where
        T: fmt::Display,
    {
        // DropStaleResponse(i, j, m) ==
        if msg_term < self.current_term {
            // /\ m.mterm < currentTerm[i]
            info!(
                "ignored message with {} < current {}: {}",
                &msg_term, &self.current_term, &msg
            );
            drop(msg); // /\ Discard(m)
            Ok(())
        } else {
            Err(msg)
        }
    }

    // /* Receive a message.
    pub fn receive(&mut self, msg: Message, from: NodeId) -> Option<SendableMessage<NodeId>> {
        // Receive(m) ==
        if !self.peers.contains(&from) {
            error!("received raft message from {} for wrong group", &from);
            return None;
        }
        // IN \* Any RPC with a newer term causes the recipient to advance
        //    \* its term first. Responses with stale terms are ignored.
        self.update_term(&from, &msg); //    \/ UpdateTerm(i, j, m)
        let reply = match msg.rpc {
            Some(Rpc::VoteRequest(request)) =>
            //    \/ /\ m.mtype = RequestVoteRequest
            {
                self.handle_vote_request(msg.term, request, from)
            } //       /\ HandleRequestVoteRequest(i, j, m)
            Some(Rpc::VoteResponse(response)) => {
                //    \/ /\ m.mtype = RequestVoteResponse
                match self.drop_stale_response(msg.term, response) {
                    //       /\ \/ DropStaleResponse(i, j, m)
                    Ok(()) => None,
                    Err(response) => self.handle_vote_response(msg.term, response, from), //          \/ HandleRequestVoteResponse(i, j, m)
                }
            }
            Some(Rpc::AppendRequest(request)) =>
            //    \/ /\ m.mtype = AppendEntriesRequest
            {
                self.handle_append_request(msg.term, request, from)
            } //       /\ HandleAppendEntriesRequest(i, j, m)
            Some(Rpc::AppendResponse(response)) => {
                //    \/ /\ m.mtype = AppendEntriesResponse
                match self.drop_stale_response(msg.term, response) {
                    //       /\ \/ DropStaleResponse(i, j, m)
                    Ok(()) => None,
                    Err(response) => self.handle_append_response(msg.term, response, from), //          \/ HandleAppendEntriesResponse(i, j, m)
                }
            }
            None => None,
        };
        self.become_leader();
        self.advance_commit_idx();
        reply
    }

    //
    // helpers
    //

    fn quorum_size(&self) -> usize {
        quorum_size(self.peers.len())
    }

    fn random_election_timeout(&mut self) -> u32 {
        random_election_timeout(&mut self.random, self.config.election_timeout_ticks)
    }
}

/// Computes the minimum size of a quorum of nodes in a Raft group.
///
/// Returns the minimum number of nodes out of a Raft group with total `peer_count` nodes necessary to constitute a
/// quorum. A quorum of reachable nodes is needed to elect a leader and append to the distributed log.
pub fn quorum_size(peer_count: usize) -> usize {
    (peer_count.saturating_add(1)) / 2 + 1
}

fn random_election_timeout(random: &mut impl RngCore, election_timeout_ticks: u32) -> u32 {
    let random = random
        .next_u32()
        .checked_rem(election_timeout_ticks)
        .unwrap_or(0);
    election_timeout_ticks.saturating_add(random)
}
