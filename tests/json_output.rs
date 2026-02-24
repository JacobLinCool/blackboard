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
fn json_flag_wraps_success_output() {
    let env = TestEnv::new();

    let out = env.run_ok(&["init", "--user", "alice", "--json"]);
    let value: serde_json::Value = serde_json::from_str(out.trim()).expect("invalid json output");

    assert_eq!(value["ok"], serde_json::Value::Bool(true));
    let lines = value["lines"].as_array().expect("lines should be an array");
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0]
            .as_str()
            .expect("line should be string")
            .contains("initialized user alice")
    );
}

#[test]
fn json_flag_wraps_error_output() {
    let env = TestEnv::new();

    let (stdout, stderr) = env.run_fail_output(&["board", "list", "--user", "ghost", "--json"]);
    assert!(stderr.trim().is_empty());

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("invalid error json output");
    assert_eq!(value["ok"], serde_json::Value::Bool(false));
    assert!(
        value["error"]
            .as_str()
            .expect("error should be string")
            .contains("auth: user 'ghost' not found")
    );
}

#[test]
fn json_flag_is_global_after_subcommand() {
    let env = TestEnv::new();

    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    let out = env.run_ok(&["board", "list", "--user", "alice", "--json"]);
    let value: serde_json::Value = serde_json::from_str(out.trim()).expect("invalid json output");

    assert_eq!(value["ok"], serde_json::Value::Bool(true));
    let lines = value["lines"].as_array().expect("lines should be an array");
    assert!(
        lines
            .iter()
            .any(|line| line.as_str().unwrap_or_default().contains("alpha"))
    );
}

#[test]
fn clear_json_output_matches_contract() {
    let env = TestEnv::new();

    if is_system_root() {
        env.run_ok(&["init", "--user", "alice"]);
        let out = env.run_ok(&["clear", "--json"]);
        let value: serde_json::Value =
            serde_json::from_str(out.trim()).expect("invalid clear json output");
        assert_eq!(value["ok"], serde_json::Value::Bool(true));
        let lines = value["lines"].as_array().expect("lines should be an array");
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0]
                .as_str()
                .expect("line should be string")
                .contains("cleared all data by removing")
        );
    } else {
        let (stdout, stderr) = env.run_fail_output(&["clear", "--json"]);
        assert!(stderr.trim().is_empty());
        let value: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("invalid clear error json output");
        assert_eq!(value["ok"], serde_json::Value::Bool(false));
        assert!(
            value["error"]
                .as_str()
                .expect("error should be string")
                .contains("forbidden: clear requires system root privileges")
        );
    }
}
