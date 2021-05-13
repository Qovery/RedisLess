use common::*;
use raft::message::{Rpc, TermId};

mod common;

#[test]
pub fn append_request_update_leader() {
    let mut raft = raft(1, vec![2], None, &mut init_random());
    assert!(!raft.is_leader());
    let (_, &(mut term)) = raft.leader();
    term += 1;

    send(&mut raft, 2, term, Rpc::AppendRequest(Default::default()));
    assert_eq!(raft.leader(), (Some(&2.into()), &term));
}

#[test]
pub fn no_update_leader() {
    for rpc in rpc_types()
        .iter()
        .cloned()
        .filter(|rpc| !matches!(rpc, Rpc::AppendRequest(_)))
    {
        let mut raft = raft(1, vec![2, 3], None, &mut init_random());
        let mut term = TermId::default();
        assert_eq!(raft.leader(), (None, &term));

        term += 1;
        send(&mut raft, 2, term, rpc);
        assert_eq!(raft.leader(), (None, &term));
    }
}
