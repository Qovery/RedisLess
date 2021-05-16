# Raft

## Problem and Importance

When building a distributed system one principal goal is often to build in *fault-tolerance*. That is, if one particular node in a network
goes down, or if there is a network partition, the entire cluster does not fall over. The cluster of nodes taking part in a distributed
consensus protocol must come to agreement regarding values, and once that decision is reached, that choice is final.

Distributed Consensus Algorithms often take the form of a replicated state machine and log. Each state machine accepts inputs from its log,
and represents the value(s) to be replicated, for example, a hash table. They allow a collection of machines to work as a coherent group
that can survive the failures of some of its members.

Two well known Distributed Consensus Algorithms are Paxos and Raft. Paxos is used in systems
like [Chubby](http://research.google.com/archive/chubby.html) by Google, and Raft is used in things
like [`tikv`](https://github.com/tikv/tikv) or [`etcd`](https://github.com/coreos/etcd/tree/master/raft). Raft is generally seen as a more
understandable and simpler to implement than Paxos.

## Documentation

* [The Raft site](https://raftconsensus.github.io/)
* [The Secret Lives of Data - Raft](http://thesecretlivesofdata.com/raft/)
* [Raft Paper](http://ramcloud.stanford.edu/raft.pdf)
* [Raft Dissertation](https://github.com/ongardie/dissertation#readme)
* [Raft Refloated](https://www.cl.cam.ac.uk/~ms705/pub/papers/2015-osr-raft.pdf)
* [Raft TLA+](https://github.com/ongardie/raft.tla/blob/master/raft.tla)

## Credits

The original raft implementation comes from [raft-rs](https://github.com/simple-raft-rs/raft-rs)
