use common::*;
use raft::message::{Message, Rpc, TermId, VoteResponse};

mod common;

#[test]
pub fn leader_update_term() {
    for rpc in rpc_types().iter().cloned() {
        let mut raft = raft(1, vec![2, 3], None, &mut init_random());
        let mut term = TermId::default();
        assert_eq!(raft.leader().1, &term);

        term += 1;
        let Message { term: new_term, .. } = raft.timeout().unwrap().message;
        assert_eq!(new_term, term);
        assert_eq!(raft.leader().1, &term);

        send(
            &mut raft,
            2,
            term,
            Rpc::VoteResponse(VoteResponse { vote_granted: true }),
        );
        assert_eq!(raft.leader(), (Some(raft.node_id()), &term));

        term += 1;
        send(&mut raft, 2, term, rpc);
        assert_eq!(raft.leader().1, &term);
    }
}

#[test]
pub fn candidate_update_term() {
    for rpc in rpc_types().iter().cloned() {
        let mut raft = raft(1, vec![2, 3], None, &mut init_random());
        let mut term = TermId::default();
        assert_eq!(raft.leader().1, &term);

        term += 1;
        let Message { term: new_term, .. } = raft.timeout().unwrap().message;
        assert_eq!(new_term, term);
        assert_eq!(raft.leader(), (None, &term));

        term += 1;
        send(&mut raft, 2, term, rpc);
        assert_eq!(raft.leader().1, &term);
    }
}

#[test]
pub fn follower_update_term() {
    for rpc in rpc_types().iter().cloned() {
        let mut raft = raft(1, vec![2, 3], None, &mut init_random());
        let mut term = TermId::default();
        assert_eq!(raft.leader(), (None, &term));

        term += 1;
        send(&mut raft, 2, term, rpc);
        assert_eq!(raft.leader().1, &term);
    }
}
