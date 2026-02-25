mod common;

use std::process::Command;
use std::sync::{Arc, Barrier};
use std::thread;

use common::TestEnv;

#[test]
fn concurrent_task_add_calls_succeed_without_lock_errors() {
    let env = TestEnv::new();
    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    let workers = 16usize;
    let gate = Arc::new(Barrier::new(workers));
    let mut handles = Vec::new();

    for i in 0..workers {
        let gate = Arc::clone(&gate);
        let bin = env.bin_path().to_path_buf();
        let home = env.home_path().to_path_buf();
        handles.push(thread::spawn(move || {
            gate.wait();
            let title = format!("t{i}");
            let output = Command::new(bin)
                .env("HOME", home)
                .args([
                    "task",
                    "add",
                    "--user",
                    "alice",
                    "--board",
                    "alpha",
                    "--title",
                    &title,
                    "--description",
                    "load",
                    "--size",
                    "small",
                ])
                .output()
                .expect("failed to run task add");
            (
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
        }));
    }

    for handle in handles {
        let (ok, stdout, stderr) = handle.join().expect("worker thread panicked");
        assert!(ok, "command failed\nstdout:\n{stdout}\nstderr:\n{stderr}");
        assert!(stdout.contains("created task"));
    }

    let out = env.run_ok(&["task", "list", "--user", "alice", "--board", "alpha"]);
    let task_rows = out
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(task_rows, workers);
}

#[test]
fn concurrent_permission_updates_remain_atomic() {
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

    let gate = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for permissions in ["read,create,update", "read,delete,assign"] {
        let gate = Arc::clone(&gate);
        let bin = env.bin_path().to_path_buf();
        let home = env.home_path().to_path_buf();
        let permissions = permissions.to_string();
        handles.push(thread::spawn(move || {
            gate.wait();
            let output = Command::new(bin)
                .env("HOME", home)
                .args([
                    "board",
                    "grant",
                    "--user",
                    "alice",
                    "--board",
                    "alpha",
                    "--target",
                    "bob",
                    "--permissions",
                    &permissions,
                ])
                .output()
                .expect("failed to run board grant");
            (
                output.status.success(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
        }));
    }

    for handle in handles {
        let (ok, stderr) = handle.join().expect("worker thread panicked");
        assert!(ok, "concurrent grant failed\nstderr:\n{stderr}");
    }

    let out = env.run_ok(&["board", "members", "--user", "alice", "--board", "alpha"]);
    let bob_line = out
        .lines()
        .find(|line| line.contains("bob [member]"))
        .expect("missing bob member line");
    assert!(
        bob_line.ends_with("perms=read,create,update")
            || bob_line.ends_with("perms=read,delete,assign"),
        "unexpected permission shape after concurrent updates: {bob_line}"
    );
}
