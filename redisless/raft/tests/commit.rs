use common::*;

mod common;

#[test]
pub fn _1_commit() {
    TestRaftGroup::new(1, &mut init_random(), config())
        .run_until(|group| group.has_leader())
        .modify(|group| {
            assert!(group
                .nodes
                .iter_mut()
                .any(|raft| raft.client_request("one".into()).is_ok()))
        })
        .run_until_commit(|commit| {
            assert_eq!(commit.data, "one");
            true
        });
}

#[test]
pub fn _2_commit() {
    TestRaftGroup::new(2, &mut init_random(), config())
        .run_until(|group| group.has_leader())
        .modify(|group| {
            assert!(group
                .nodes
                .iter_mut()
                .any(|raft| raft.client_request("one".into()).is_ok()))
        })
        .run_until_commit(|commit| {
            assert_eq!(commit.data, "one");
            true
        });
}

#[test]
pub fn _3_commit() {
    TestRaftGroup::new(3, &mut init_random(), config())
        .run_until(|group| group.has_leader())
        .modify(|group| {
            assert!(group
                .nodes
                .iter_mut()
                .any(|raft| raft.client_request("one".into()).is_ok()))
        })
        .run_until_commit(|commit| {
            assert_eq!(commit.data, "one");
            true
        });
}

#[test]
pub fn commit_leader_change() {
    let mut group = TestRaftGroup::new(3, &mut init_random(), config());
    group.run_on_node(0, |raft| raft.timeout());
    group.run_until(|group| group.nodes[0].is_leader());

    assert!(group.nodes[0].client_request("one".into()).is_ok());
    group.config = config().drop_to(0);
    group.run_for(1);

    assert!(group.take_committed().all(|commit| commit.data.is_empty()));
    group.config = config().isolate(0);
    group.run_until_commit(|commit| {
        assert_eq!(commit.data, "one");
        true
    });
}

#[test]
pub fn cancel_uncommitted() {
    let mut group = TestRaftGroup::new(3, &mut init_random(), config());
    group.run_on_node(0, |raft| raft.timeout());
    group.run_until(|group| group.nodes[0].is_leader());

    assert!(group.nodes[0].client_request("one".into()).is_ok());
    group.config = config().isolate(0);
    group.run_until(|group| group.nodes[1..].iter().any(|raft| raft.is_leader()));

    assert!(group.nodes[1..]
        .iter_mut()
        .any(|raft| raft.client_request("two".into()).is_ok()));
    group.run_until_commit(|commit| {
        assert_eq!(commit.data, "two");
        true
    });

    log::info!("committed two");
    group.config = config();
    group.run_until(|group| {
        group.nodes[0].take_committed().any(|commit| {
            if !commit.data.is_empty() {
                assert_eq!(commit.data, "two");
                true
            } else {
                false
            }
        })
    });
}
