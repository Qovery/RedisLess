#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};

use rand_core::{RngCore, SeedableRng};

use raft::core::State;
use raft::log::memory::InMemoryLog;
use raft::message::{LogEntry, Message, MessageDestination, Rpc, SendableMessage, TermId};
use raft::node::Config;
use rand_chacha::ChaChaRng;

pub const CONFIG: Config = Config {
    election_timeout_ticks: 10,
    heartbeat_interval_ticks: 9,
    replication_chunk_size: 1024,
};
const RANDOM_SEED: u64 = 0;
const MAX_TICKS: u32 = 100_000;

pub type TestRaft = State<InMemoryLog, ChaChaRng, NodeId>;

pub struct TestRaftGroup {
    pub nodes: Vec<TestRaft>,
    pub tick: u32,
    pub config: TestRaftGroupConfig,
    pub dropped_messages: Vec<(NodeId, SendableMessage<NodeId>)>,
}

#[derive(Clone, Default)]
pub struct TestRaftGroupConfig {
    pub drops: BTreeSet<(Option<NodeId>, Option<NodeId>)>,
    pub down: BTreeSet<NodeId>,
}

#[derive(
    Clone, Copy, Debug, derive_more::Display, Eq, derive_more::From, PartialEq, PartialOrd, Ord,
)]
#[display(fmt = "{:?}", self)]
pub struct NodeId(u64);

pub struct TestLogger;

pub struct TestLoggerContext {
    node_id: Option<NodeId>,
    tick: Option<u32>,
}

pub fn rpc_types() -> [Rpc; 4] {
    [
        Rpc::VoteRequest(Default::default()),
        Rpc::VoteResponse(Default::default()),
        Rpc::AppendRequest(Default::default()),
        Rpc::AppendResponse(Default::default()),
    ]
}

pub fn init_random() -> ChaChaRng {
    ChaChaRng::seed_from_u64(RANDOM_SEED)
}

pub fn raft(
    node_id: u64,
    peers: Vec<u64>,
    log: Option<InMemoryLog>,
    random: &mut impl RngCore,
) -> TestRaft {
    TestLogger::init();
    State::new(
        NodeId(node_id),
        peers.into_iter().map(NodeId).collect(),
        log.unwrap_or_else(|| InMemoryLog::new_unbounded()),
        ChaChaRng::seed_from_u64(random.next_u64()),
        CONFIG,
    )
}

pub fn config() -> TestRaftGroupConfig {
    TestRaftGroupConfig::default()
}

pub fn send(
    raft: &mut TestRaft,
    from: u64,
    term: TermId,
    rpc: Rpc,
) -> Option<SendableMessage<NodeId>> {
    raft.receive(
        Message {
            term,
            rpc: Some(rpc),
        },
        NodeId(from),
    )
}

pub fn append_entries<'a>(
    node: &'a mut TestRaft,
    peers: impl IntoIterator<Item = NodeId> + 'a,
) -> impl Iterator<Item = SendableMessage<NodeId>> + 'a {
    let node_id = *node.node_id();
    peers.into_iter().flat_map(move |append_to_node_id| {
        if append_to_node_id != node_id {
            node.append_entries(append_to_node_id)
        } else {
            None
        }
    })
}

pub fn run_group<'a>(
    nodes: impl Iterator<Item = &'a mut TestRaft> + ExactSizeIterator,
    initial_messages: impl IntoIterator<Item = (NodeId, SendableMessage<NodeId>)>,
    start_tick: u32,
    ticks: Option<u32>,
    config: &mut TestRaftGroupConfig,
    dropped_messages: &mut Vec<(NodeId, SendableMessage<NodeId>)>,
) {
    let mut nodes: Vec<_> = nodes.collect();
    let node_ids: Vec<_> = nodes.iter().map(|node| *node.node_id()).collect();
    let mut messages = VecDeque::with_capacity(nodes.len() * nodes.len());
    messages.extend(initial_messages.into_iter());
    messages.extend(dropped_messages.drain(..));

    for tick in 0..ticks.unwrap_or(1) {
        TestLogger::set_tick(Some(start_tick + tick));
        if ticks.is_some() {
            for node in &mut nodes {
                let node_id = *node.node_id();
                if !config.is_node_down(node_id) {
                    TestLogger::set_node_id(Some(node_id));
                    messages.extend(node.timer_tick().map(|message| (node_id, message)));
                    messages.extend(
                        append_entries(node, node_ids.iter().cloned())
                            .map(|message| (node_id, message)),
                    );
                }
            }
        }

        while let Some((from, sendable)) = messages.pop_front() {
            let (reply_to_node_id, to_node_count) = match sendable.dest {
                MessageDestination::Broadcast => (None, nodes.len().saturating_sub(1)),
                MessageDestination::To(to) => (Some(to), 1),
            };
            let to_nodes = nodes.iter_mut().filter(|node| match &reply_to_node_id {
                Some(to_node_id) => node.node_id() == to_node_id,
                None => node.node_id() != &from,
            });

            for (to_node, message) in Iterator::zip(
                to_nodes,
                itertools::repeat_n(sendable.message, to_node_count),
            ) {
                let to_node_id = *to_node.node_id();
                TestLogger::set_node_id(Some(to_node_id));
                if !config.should_drop(from, to_node_id) {
                    log::info!("<- {} {}", from, message);
                    messages.extend(
                        to_node
                            .receive(message, from)
                            .map(|message| (to_node_id, message)),
                    );
                } else {
                    log::info!("<- {} DROPPED {}", from, message);
                    if let Some(reply_to_node_id) = reply_to_node_id {
                        dropped_messages.push((
                            from,
                            SendableMessage {
                                message,
                                dest: MessageDestination::To(reply_to_node_id),
                            },
                        ));
                    }
                }
                messages.extend(
                    append_entries(to_node, node_ids.iter().cloned())
                        .map(|message| (to_node_id, message)),
                );
            }
        }
    }
    TestLogger::set_tick(None);
    TestLogger::set_node_id(None);
}

//
// RaftGroup impls
//

impl TestRaftGroup {
    pub fn new(size: u64, random: &mut impl RngCore, config: TestRaftGroupConfig) -> Self {
        let nodes: Vec<u64> = (0..size).collect();
        Self {
            nodes: nodes
                .iter()
                .map(|node_id| raft(*node_id, nodes.clone(), None, random))
                .collect(),
            tick: 0,
            config,
            dropped_messages: Default::default(),
        }
    }

    pub fn run_until(&mut self, mut until_fun: impl FnMut(&mut Self) -> bool) -> &mut Self {
        let mut ticks_remaining = MAX_TICKS;
        while !until_fun(self) {
            ticks_remaining = ticks_remaining
                .checked_sub(1)
                .expect("condition failed after maximum simulation length");
            self.tick += 1;
            run_group(
                self.nodes.iter_mut(),
                None,
                self.tick,
                Some(1),
                &mut self.config,
                &mut self.dropped_messages,
            );
        }
        self
    }

    pub fn run_until_commit(&mut self, mut until_fun: impl FnMut(&LogEntry) -> bool) -> &mut Self {
        self.run_until(|group| {
            let result = group
                .take_committed()
                .any(|commit| !commit.data.is_empty() && until_fun(&commit));
            group.take_committed().for_each(drop);
            result
        })
    }

    pub fn run_for(&mut self, ticks: u32) -> &mut Self {
        self.run_for_inspect(ticks, |_| ())
    }

    pub fn run_for_inspect(&mut self, ticks: u32, mut fun: impl FnMut(&mut Self)) -> &mut Self {
        let mut ticks_remaining = ticks;
        while let Some(new_ticks_remaining) = ticks_remaining.checked_sub(1) {
            ticks_remaining = new_ticks_remaining;
            self.tick += 1;
            run_group(
                self.nodes.iter_mut(),
                None,
                self.tick,
                Some(1),
                &mut self.config,
                &mut self.dropped_messages,
            );
            fun(self);
        }
        self
    }

    pub fn run_on_all(
        &mut self,
        mut fun: impl FnMut(&mut TestRaft) -> Option<SendableMessage<NodeId>>,
    ) -> &mut Self {
        let messages = self
            .nodes
            .iter_mut()
            .flat_map(|node| fun(node).map(|message| (*node.node_id(), message)))
            .collect::<Vec<_>>();
        run_group(
            self.nodes.iter_mut(),
            messages,
            self.tick,
            None,
            &mut self.config,
            &mut self.dropped_messages,
        );
        self
    }

    pub fn run_on_node(
        &mut self,
        node_idx: usize,
        fun: impl FnOnce(&mut TestRaft) -> Option<SendableMessage<NodeId>>,
    ) -> &mut Self {
        let node_id = *self.nodes[node_idx].node_id();
        let messages = fun(&mut self.nodes[node_idx]).map(|message| (node_id, message));
        run_group(
            self.nodes.iter_mut(),
            messages,
            self.tick,
            None,
            &mut self.config,
            &mut self.dropped_messages,
        );
        self
    }

    pub fn inspect(&mut self, fun: impl FnOnce(&Self)) -> &mut Self {
        fun(self);
        self
    }

    pub fn modify(&mut self, fun: impl FnOnce(&mut Self)) -> &mut Self {
        fun(self);
        self
    }

    pub fn take_committed(&mut self) -> impl Iterator<Item = LogEntry> + '_ {
        self.nodes.iter_mut().flat_map(|node| node.take_committed())
    }

    pub fn has_leader(&self) -> bool {
        self.nodes.iter().any(|node| node.is_leader())
    }
}

//
// TestRaftGroupConfig impls
//

impl TestRaftGroupConfig {
    pub fn node_down(mut self, node_id: u64) -> Self {
        self.down.insert(NodeId(node_id));
        self
    }

    pub fn isolate(mut self, node_id: u64) -> Self {
        self.drops.insert((Some(NodeId(node_id)), None));
        self.drops.insert((None, Some(NodeId(node_id))));
        self
    }

    pub fn drop_between(mut self, from: u64, to: u64) -> Self {
        self.drops.insert((Some(NodeId(from)), Some(NodeId(to))));
        self.drops.insert((Some(NodeId(to)), Some(NodeId(from))));
        self
    }

    pub fn drop_to(mut self, node_id: u64) -> Self {
        self.drops.insert((None, Some(NodeId(node_id))));
        self
    }

    pub fn is_node_down(&self, node_id: NodeId) -> bool {
        self.down.contains(&node_id)
    }

    pub fn should_drop(&self, from: NodeId, to: NodeId) -> bool {
        self.drops.contains(&(Some(from), Some(to)))
            || self.drops.contains(&(Some(from), None))
            || self.drops.contains(&(None, Some(to)))
            || self.down.contains(&from)
            || self.down.contains(&to)
    }
}

//
// TestLogger impls
//

thread_local! {
    static LOGGER_CONTEXT: RefCell<TestLoggerContext> = RefCell::new(TestLoggerContext::new());
}

impl TestLogger {
    pub fn init() {
        let _ignore = log::set_logger(&Self);
        log::set_max_level(log::LevelFilter::Debug);
    }
    pub fn set_node_id(node_id: Option<NodeId>) {
        LOGGER_CONTEXT.with(|context| {
            context.borrow_mut().node_id = node_id;
        });
    }
    pub fn set_tick(tick: Option<u32>) {
        LOGGER_CONTEXT.with(|context| {
            context.borrow_mut().tick = tick;
        });
    }
}

impl log::Log for TestLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        LOGGER_CONTEXT.with(|context| {
            let context = context.borrow();
            if let Some(node_id) = context.node_id {
                if let Some(tick) = context.tick {
                    eprintln!("tick {:03} {} {}", tick, node_id, record.args());
                } else {
                    eprintln!("tick ??? {} {}", node_id, record.args());
                }
            } else {
                eprintln!("{}", record.args());
            }
        })
    }

    fn flush(&self) {}
}

//
// TextLoggerContext impls
//

impl TestLoggerContext {
    const fn new() -> Self {
        Self {
            node_id: None,
            tick: None,
        }
    }
}
