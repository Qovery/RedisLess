//! A naive in-memory implementation of [`RaftLog`](super::RaftLog), primarily for testing.

use alloc::collections::VecDeque;
use core::convert::{TryFrom, TryInto};

use crate::message::{LogEntry, LogIndex, TermId};

use super::Log;

/// A naive in-memory implementation of [`Log`](super::Log), primarily for testing.
pub struct InMemoryLog {
    entries: VecDeque<LogEntry>,
    prev_log_idx: LogIndex,
    prev_log_term: TermId,
    last_taken: LogIndex,
    data_len: usize,
    data_capacity: usize,
}

impl InMemoryLog {
    /// Constructs an empty Raft log with unbounded capacity.
    pub fn new_unbounded() -> Self {
        Self::with_capacity(0, usize::max_value())
    }

    /// Constructs an empty Raft log with bounded capacity.
    ///
    /// `initial_entries_capacity` specifies how many log entries the Raft log will be able to store without
    /// reallocating. `data_capacity` specifies the maximum size of log entry data to store before discarding entries
    /// from the beginning of the log.
    pub fn with_capacity(initial_entries_capacity: usize, data_capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(initial_entries_capacity),
            prev_log_idx: LogIndex::default(),
            prev_log_term: TermId::default(),
            last_taken: LogIndex::default(),
            data_len: 0,
            data_capacity,
        }
    }

    fn entry_index(&self, log_idx: LogIndex) -> Option<usize> {
        log_idx
            .id
            .checked_sub(self.prev_log_idx.id)?
            .checked_sub(1)?
            .try_into()
            .ok()
    }

    fn pop_front(&mut self) -> Result<(), <Self as Log>::Error> {
        self.entry_index(self.last_taken).ok_or(())?;
        let prev_log = self.entries.pop_front().ok_or(())?;
        self.prev_log_idx = self.prev_log_idx + 1;
        self.prev_log_term = prev_log.term;
        Ok(())
    }
}

impl Log for InMemoryLog {
    type Error = ();

    fn append(&mut self, log_entry: LogEntry) -> Result<(), Self::Error> {
        if log_entry.data.len() > self.data_capacity {
            return Err(());
        }

        self.data_len = loop {
            match self.data_len.checked_add(log_entry.data.len()) {
                Some(new_data_len) if new_data_len <= self.data_capacity => break new_data_len,
                Some(_) | None => {
                    self.pop_front()?;
                }
            }
        };

        self.entries.push_back(log_entry);
        Ok(())
    }

    fn cancel_from(&mut self, from_log_idx: LogIndex) -> Result<usize, ()> {
        let from_index = self.entry_index(from_log_idx).ok_or(())?;
        match self.entries.len().checked_sub(from_index) {
            Some(0) | None => Err(()),
            Some(cancelled_len) => {
                self.entries.truncate(from_index);
                Ok(cancelled_len)
            }
        }
    }

    fn entry_len(&self, log_entry: &LogEntry) -> usize {
        4 + log_entry.data.len()
    }

    fn get(&mut self, log_idx: LogIndex) -> Option<LogEntry> {
        let index = self.entry_index(log_idx)?;
        self.entries.get(index).cloned()
    }

    fn get_term(&mut self, log_idx: LogIndex) -> Option<TermId> {
        if log_idx != self.prev_log_idx {
            self.get(log_idx).map(|log_entry: LogEntry| log_entry.term)
        } else {
            Some(self.prev_log_term)
        }
    }

    fn prev_index(&self) -> LogIndex {
        self.prev_log_idx
    }

    fn last_index(&self) -> LogIndex {
        let entries_len = u64::try_from(self.entries.len())
            .unwrap_or_else(|_| panic!("more than 2^64 log entries"));
        self.prev_log_idx + entries_len
    }

    fn last_taken_index(&self) -> LogIndex {
        self.last_taken
    }

    fn last_term(&self) -> TermId {
        self.entries
            .iter()
            .map(|log_entry: &LogEntry| log_entry.term)
            .last()
            .unwrap_or(self.prev_log_term)
    }

    fn prev_term(&self) -> TermId {
        self.prev_log_term
    }

    fn take_next(&mut self) -> Option<LogEntry> {
        let log_idx = self.last_taken + 1;
        let log_entry = self.get(log_idx)?;
        self.last_taken = log_idx;
        Some(log_entry)
    }
}

#[cfg(test)]
mod test {
    use crate::raft_log_tests;

    use super::*;

    raft_log_tests!(InMemoryLog, InMemoryLog::new_unbounded());
}
