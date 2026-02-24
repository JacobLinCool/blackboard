mod common;

use common::TestEnv;

#[cfg(unix)]
fn is_system_root() -> bool {
    unsafe {
        // SAFETY: libc::geteuid has no preconditions and does not dereference pointers.
        libc::geteuid() == 0
    }
}

#[cfg(not(unix))]
fn is_system_root() -> bool {
    false
}

#[test]
fn clear_enforces_system_root_and_removes_db_when_allowed() {
    let env = TestEnv::new();
    env.run_ok(&["init", "--user", "alice"]);
    assert!(env.db_path().exists());

    if is_system_root() {
        let out = env.run_ok(&["clear"]);
        assert!(out.contains("cleared all data by removing"));
        assert!(!env.db_path().exists());
    } else {
        let err = env.run_err(&["clear"]);
        assert!(err.contains("forbidden: clear requires system root privileges"));
        assert!(env.db_path().exists());
    }
}
