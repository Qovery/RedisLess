use bytes::Bytes;

use crate::message::{LogEntry, LogIndex, TermId};

use super::Log;

/// Defines test functions for a type implementing RaftLog.
#[macro_export]
macro_rules! raft_log_tests {
    ($ty:ty, $new:expr) => {
        $crate::raft_log_test! { $ty, $new, test_log_empty }
        $crate::raft_log_test! { $ty, $new, test_log_append }
        $crate::raft_log_test! { $ty, $new, test_log_cancel_from }
    };
}

/// Defines a given test function for a type implementing RaftLog.
#[macro_export]
macro_rules! raft_log_test {
    ($ty:ty, $new:expr, $test:ident) => {
        #[test]
        fn $test() {
            let mut log: $ty = $new;
            $crate::log::tests::$test(&mut log);
        }
    };
}

pub fn test_log_empty<L: Log>(log: &mut L) {
    verify_log(log, &[], LogIndex::default(), LogIndex::default());
}

pub fn test_log_append<L: Log>(log: &mut L) {
    let entries = test_entries();
    for (index, entry) in entries.iter().cloned().enumerate() {
        log.append(entry).unwrap_or_else(|_| panic!());
        verify_log(
            log,
            &entries,
            LogIndex::default(),
            LogIndex {
                id: 1 + index as u64,
            },
        );
    }
}

pub fn test_log_cancel_from<L: Log>(log: &mut L) {
    let entries = append_test_entries(log);
    for &truncate_len in &[1, 2, 1] {
        let last_log_idx = log.last_index();
        log.cancel_from(last_log_idx + 2).unwrap_err();
        log.cancel_from(last_log_idx + 1).unwrap_err();
        verify_log(log, &entries, LogIndex::default(), last_log_idx);
        assert_eq!(
            log.cancel_from(last_log_idx + 1 - truncate_len)
                .map_err(drop),
            Ok(truncate_len as usize)
        );
        verify_log(
            log,
            &entries,
            LogIndex::default(),
            last_log_idx - truncate_len,
        );
    }
    log.cancel_from(log.last_index() + 2).unwrap_err();
    log.cancel_from(log.last_index() + 1).unwrap_err();
}

//
// internal
//

fn test_entries() -> [LogEntry; 5] {
    [
        LogEntry {
            term: TermId { id: 1 },
            data: Bytes::from_static(&[]),
        },
        LogEntry {
            term: TermId { id: 1 },
            data: Bytes::from_static(&[2; 1]),
        },
        LogEntry {
            term: TermId { id: 2 },
            data: Bytes::from_static(&[3; 2]),
        },
        LogEntry {
            term: TermId { id: 9 },
            data: Bytes::from_static(&[4; 100]),
        },
        LogEntry {
            term: TermId {
                id: u64::max_value(),
            },
            data: Bytes::from_static(&[5; 100]),
        },
    ]
}

fn append_test_entries<L: Log>(log: &mut L) -> [LogEntry; 5] {
    let entries = test_entries();
    entries
        .iter()
        .cloned()
        .for_each(|entry| log.append(entry).unwrap_or_else(|_| panic!()));
    entries
}

fn verify_log<L: Log>(
    log: &mut L,
    entries: &[LogEntry],
    prev_log_idx: LogIndex,
    last_log_idx: LogIndex,
) {
    assert_eq!(log.prev_index(), prev_log_idx);

    assert_eq!(log.get(LogIndex::default()), None);
    assert_eq!(log.get_len(LogIndex::default()), None);

    assert_eq!(log.get(prev_log_idx), None);
    assert_eq!(
        log.get_term(prev_log_idx),
        Some(
            prev_log_idx
                .id
                .checked_sub(1)
                .map(|index| entries[index as usize].term)
                .unwrap_or_default()
        )
    );
    assert_eq!(log.get_len(prev_log_idx), None);

    assert_eq!(log.last_index(), last_log_idx);
    assert_eq!(
        log.last_term(),
        log.last_index()
            .id
            .checked_sub(1)
            .map(|index| entries[index as usize].term)
            .unwrap_or_default()
    );

    verify_entries(entries, prev_log_idx, last_log_idx, |log_idx, entry| {
        assert_eq!(log.get(log_idx).as_ref(), entry);
        assert_eq!(log.get_term(log_idx), entry.map(|entry| entry.term));
        assert_eq!(
            log.get_len(log_idx),
            entry.map(|entry| log.entry_len(&entry))
        );
    });
}

fn verify_entries<F>(
    entries: &[LogEntry],
    prev_log_idx: LogIndex,
    last_log_idx: LogIndex,
    mut fun: F,
) where
    F: FnMut(LogIndex, Option<&LogEntry>),
{
    for log_index in 0..prev_log_idx.id {
        fun(LogIndex { id: log_index }, None);
    }
    for entry_index in prev_log_idx.id..last_log_idx.id {
        fun(
            LogIndex {
                id: 1 + entry_index,
            },
            Some(&entries[entry_index as usize]),
        );
    }
    for entry_index in last_log_idx.id..=entries.len() as u64 {
        fun(
            LogIndex {
                id: 1 + entry_index,
            },
            None,
        );
    }
}
