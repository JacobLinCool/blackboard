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
        "--size",
        "small",
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
        "--size",
        "small",
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
        "--size",
        "small",
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
        "--size",
        "small",
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
        "--size",
        "small",
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
        "--size",
        "small",
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
        "--size",
        "small",
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
        "--size",
        "small",
        "--depends-on",
        "a,2",
    ]);
    assert!(err.contains("input: invalid task id 'a'"));
}

#[test]
fn status_transitions_require_completed_dependencies_and_required_notes() {
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
        "base",
        "--size",
        "small",
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
        "dependent",
        "--size",
        "small",
        "--depends-on",
        "1",
    ]);

    let err = env.run_err(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
        "--status",
        "in_progress",
    ]);
    assert!(err.contains("cannot move to in_progress because dependency 1 is pending"));

    let out = env.run_ok(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--status",
        "in_progress",
    ]);
    assert!(out.contains("status updated"));

    let err = env.run_err(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--status",
        "completed",
    ]);
    assert!(err.contains("--note is required when status is completed"));

    env.run_ok(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--status",
        "completed",
        "--note",
        "done",
    ]);

    env.run_ok(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
        "--status",
        "in_progress",
        "--note",
        "started",
    ]);

    let err = env.run_err(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
        "--status",
        "blocked",
    ]);
    assert!(err.contains("--note is required when status is blocked"));

    env.run_ok(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
        "--status",
        "blocked",
        "--note",
        "waiting for upstream",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
    ]);
    assert!(out.contains("postNotes:"));
    assert!(out.contains("started"));
    assert!(out.contains("waiting for upstream"));
}

#[test]
fn large_tasks_are_auto_managed_from_children() {
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
        "epic",
        "--description",
        "parent",
        "--size",
        "large",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "slice-a",
        "--description",
        "a",
        "--size",
        "small",
        "--parent",
        "1",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "slice-b",
        "--description",
        "b",
        "--size",
        "small",
        "--parent",
        "1",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
    ]);
    assert!(out.contains("dependsOn: [2, 3]"));

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
        "2",
    ]);
    assert!(err.contains("large task dependencies are managed by children"));

    let err = env.run_err(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
        "--status",
        "completed",
        "--note",
        "manual",
    ]);
    assert!(err.contains("large task status is derived from children"));

    env.run_ok(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
        "--status",
        "completed",
        "--note",
        "done a",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
    ]);
    assert!(out.contains("status: in_progress"));
    assert!(out.contains("auto-updated status"));

    env.run_ok(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "3",
        "--status",
        "blocked",
        "--note",
        "blocked b",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
    ]);
    assert!(out.contains("status: blocked"));

    env.run_ok(&[
        "task",
        "status",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "3",
        "--status",
        "completed",
        "--note",
        "done b",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
    ]);
    assert!(out.contains("status: completed"));

    env.run_ok(&[
        "task",
        "delete",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
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
        "1",
    ]);
    assert!(out.contains("dependsOn: [3]"));
}

#[test]
fn large_to_non_large_keeps_dependencies() {
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
        "epic",
        "--description",
        "parent",
        "--size",
        "large",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "slice-a",
        "--description",
        "a",
        "--size",
        "small",
        "--parent",
        "1",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "slice-b",
        "--description",
        "b",
        "--size",
        "small",
        "--parent",
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
        "1",
        "--size",
        "medium",
    ]);

    let out = env.run_ok(&[
        "task",
        "view",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "1",
    ]);
    assert!(out.contains("size: medium"));
    assert!(out.contains("dependsOn: [2, 3]"));
}

#[test]
fn parent_self_and_cycle_are_rejected() {
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
        "a",
        "--description",
        "a",
        "--size",
        "small",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "b",
        "--description",
        "b",
        "--size",
        "small",
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
        "--parent",
        "1",
    ]);
    assert!(err.contains("self parent is not allowed"));

    env.run_ok(&[
        "task",
        "edit",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--task-id",
        "2",
        "--parent",
        "1",
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
        "--parent",
        "2",
    ]);
    assert!(err.contains("parent cycle detected"));
}

#[test]
fn list_can_filter_by_size() {
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
        "small-task",
        "--description",
        "s",
        "--size",
        "small",
    ]);
    env.run_ok(&[
        "task",
        "add",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--title",
        "medium-task",
        "--description",
        "m",
        "--size",
        "medium",
    ]);

    let out = env.run_ok(&[
        "task", "list", "--user", "alice", "--board", "alpha", "--size", "medium",
    ]);
    assert!(out.contains("medium-task"));
    assert!(!out.contains("small-task"));
}
