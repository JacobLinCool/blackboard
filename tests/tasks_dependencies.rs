mod common;

use common::TestEnv;

#[test]
fn dependency_rules_are_enforced() {
    let env = TestEnv::new();

    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "t1",
        "--description",
        "d1",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "t2",
        "--description",
        "d2",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "t3",
        "--description",
        "d3",
    ]);

    env.run_ok(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
        "--depends-on",
        "1",
    ]);
    env.run_ok(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "3",
        "--depends-on",
        "2",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "3",
    ]);
    assert!(out.contains("dependsOn: [2]"));

    let err = env.run_err(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--depends-on",
        "3",
    ]);
    assert!(err.contains("dependency: cycle detected"));

    let err = env.run_err(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--depends-on",
        "1",
    ]);
    assert!(err.contains("dependency: self dependency is not allowed"));

    let err = env.run_err(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--depends-on",
        "999",
    ]);
    assert!(err.contains("dependency: task 999 not in board 1"));

    env.run_ok(&["board", "create", "--user", "alice", "--name", "beta"]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "beta",
        "--title",
        "other",
        "--description",
        "other",
    ]);

    let err = env.run_err(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--depends-on",
        "4",
    ]);
    assert!(err.contains("dependency: task 4 not in board 1"));

    env.run_ok(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "3",
        "--clear-depends-on",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "3",
    ]);
    assert!(out.contains("dependsOn: []"));
}

#[test]
fn task_parent_and_input_validation_are_enforced() {
    let env = TestEnv::new();

    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    let err = env.run_err(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "t1",
        "--description",
        "d1",
        "--parent",
        "42",
    ]);
    assert!(err.contains("dependency: task 42 not in board 1"));

    env.run_ok(&["board", "create", "--user", "alice", "--name", "beta"]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "beta",
        "--title",
        "tb",
        "--description",
        "db",
    ]);

    let err = env.run_err(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "ta",
        "--description",
        "da",
        "--parent",
        "1",
    ]);
    assert!(err.contains("dependency: task 1 not in board 1"));

    let err = env.run_err(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "x",
        "--description",
        "x",
        "--depends-on",
        "a,2",
    ]);
    assert!(err.contains("input: invalid task id 'a'"));
}
