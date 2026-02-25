mod common;

use common::TestEnv;

#[test]
fn permissions_gate_each_action() {
    let env = TestEnv::new();

    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["user", "add", "--user", "alice", "--name", "bob"]);
    env.run_ok(&["user", "add", "--user", "alice", "--name", "carol"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read",
    ]);

    let err = env.run_err(&[
        "task",
        "add",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--title",
        "t1",
        "--description",
        "d1",
        "--size",
        "small",
    ]);
    assert!(err.contains("forbidden: create denied"));

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read,create",
    ]);

    let out = env.run_ok(&[
        "task",
        "add",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--title",
        "t1",
        "--description",
        "d1",
        "--size",
        "small",
    ]);
    assert!(out.contains("created task 1"));

    let err = env.run_err(&[
        "task",
        "edit",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--title",
        "new",
    ]);
    assert!(err.contains("forbidden: update denied"));

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read,create,update",
    ]);

    let out = env.run_ok(&[
        "task",
        "edit",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--title",
        "new",
    ]);
    assert!(out.contains("updated task 1"));

    let err = env.run_err(&[
        "task",
        "status",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--status",
        "completed",
        "--note",
        "done",
    ]);
    assert!(err.contains("forbidden: set_status denied"));

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read,create,update,set_status",
    ]);

    let out = env.run_ok(&[
        "task",
        "status",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--status",
        "completed",
        "--note",
        "done",
    ]);
    assert!(out.contains("status updated"));

    let err = env.run_err(&[
        "task",
        "delete",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--task-id",
        "1",
    ]);
    assert!(err.contains("forbidden: delete denied"));

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read,create,update,set_status,delete",
    ]);

    let out = env.run_ok(&[
        "task",
        "delete",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--task-id",
        "1",
    ]);
    assert!(out.contains("deleted 1 row(s)"));

    let err = env.run_err(&[
        "board",
        "grant",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--target",
        "carol",
        "--permissions",
        "read",
    ]);
    assert!(err.contains("forbidden: assign denied"));

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read,assign",
    ]);

    let out = env.run_ok(&[
        "board",
        "grant",
        "--user",
        "bob",
        "--board",
        "alpha",
        "--target",
        "carol",
        "--permissions",
        "read",
    ]);
    assert!(out.contains("granted"));
    assert!(out.contains("permissions: read"));

    let out = env.run_ok(&[
        "board", "revoke", "--user", "bob", "--board", "alpha", "--target", "carol",
    ]);
    assert!(out.contains("revoked 1 row(s)"));

    let err = env.run_err(&[
        "board", "revoke", "--user", "bob", "--board", "alpha", "--target", "alice",
    ]);
    assert!(err.contains("forbidden: cannot revoke board owner"));
}

#[test]
fn grant_normalizes_permission_order_and_deduplicates() {
    let env = TestEnv::new();

    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["user", "add", "--user", "alice", "--name", "bob"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    let out = env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "delete,read,assign,read",
    ]);

    assert!(out.contains("permissions: read,delete,assign"));
}

#[test]
fn delete_board_permission_gates_board_delete() {
    let env = TestEnv::new();

    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["user", "add", "--user", "alice", "--name", "bob"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read",
    ]);

    let err = env.run_err(&["board", "delete", "--user", "bob", "--board", "alpha"]);
    assert!(err.contains("forbidden: delete_board denied"));

    env.run_ok(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read,delete_board",
    ]);

    let out = env.run_ok(&["board", "delete", "--user", "bob", "--board", "alpha"]);
    assert!(out.contains("deleted 1 row(s)"));

    let out = env.run_ok(&["board", "list", "--user", "alice"]);
    assert!(!out.contains("alpha"));
}
