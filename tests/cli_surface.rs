mod common;

use common::TestEnv;

#[test]
fn board_grant_help_lists_permissions_flag() {
    let env = TestEnv::new();

    let out = env.run_ok(&["board", "grant", "--help"]);
    assert!(out.contains("--permissions"));
    assert!(out.contains("read, create, update, delete, set_status, assign, delete_board"));
}

#[test]
fn invalid_permission_value_is_rejected_by_cli() {
    let env = TestEnv::new();

    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    let err = env.run_err(&[
        "board",
        "grant",
        "--user",
        "alice",
        "--board",
        "alpha",
        "--target",
        "bob",
        "--permissions",
        "read,unknown",
    ]);
    assert!(err.contains("invalid value 'unknown'"));
}

#[test]
fn clear_help_does_not_accept_actor_user_flag() {
    let env = TestEnv::new();

    let out = env.run_ok(&["clear", "--help"]);
    assert!(!out.contains("--user"));
}
