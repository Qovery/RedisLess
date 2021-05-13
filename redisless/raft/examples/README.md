# Examples

* [Simple](simple.rs) -- A simple example from the `raft` crate's crate-level documentation.
* [Threaded](threaded.rs) -- A simple example with a thread per RaftNode.
* [`raftcat`](raftcat.rs) -- A complex networked example as a command-line tool.

## `raftcat`

`raftcat` is a command-line tool to run a networked Raft group over TCP. Lines from stdin are appended to the Raft log as log entries.
Committed log entries are written to stdout. This is a toy example, so no retry is attempted on log appends, which in a database would
normally be handled by the database client. This examples also does not persist state, so restarting a node may result in data loss or
inconsistency.
