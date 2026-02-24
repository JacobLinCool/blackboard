mod common;

use common::TestEnv;

#[test]
fn user_and_board_flow_works_end_to_end() {
    let env = TestEnv::new();

    let out = env.run_ok(&["init", "--user", "alice"]);
    assert!(out.contains("initialized user alice"));

    let out = env.run_ok(&["user", "add", "--user", "alice", "--name", "bob"]);
    assert!(out.contains("added user bob"));

    let out = env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);
    assert!(out.contains("created board alpha (id=1)"));

    let out = env.run_ok(&["board", "list", "--user", "alice"]);
    assert!(out.contains("alpha"));

    let out = env.run_ok(&[
        "board", "grant", "--user", "alice", "--board", "alpha", "--target", "bob",
    ]);
    assert!(out.contains("granted"));
    assert!(out.contains("permissions: read"));

    let out = env.run_ok(&["board", "list", "--user", "bob"]);
    assert!(out.contains("alpha"));

    let out = env.run_ok(&["board", "view", "--user", "bob", "--board", "1"]);
    assert!(out.contains("board alpha (id=1)"));

    let out = env.run_ok(&["board", "members", "--user", "alice", "--board", "alpha"]);
    assert!(out.contains("alice [owner] perms=all"));
    assert!(out.contains("bob [member] perms=read"));

    let out = env.run_ok(&[
        "board", "revoke", "--user", "alice", "--board", "alpha", "--target", "bob",
    ]);
    assert!(out.contains("revoked 1 row(s)"));

    let out = env.run_ok(&["board", "list", "--user", "bob"]);
    assert!(!out.contains("alpha"));

    let err = env.run_err(&["board", "view", "--user", "bob", "--board", "alpha"]);
    assert!(err.contains("forbidden: read denied"));
}
