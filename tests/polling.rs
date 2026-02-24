mod common;

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use common::TestEnv;

#[test]
fn board_poll_emits_idle_notice() {
    let env = TestEnv::new();
    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    let mut child = Command::new(env.bin_path())
        .env("HOME", env.home_path())
        .args([
            "board",
            "poll",
            "--user",
            "alice",
            "--board",
            "alpha",
            "--interval",
            "1",
            "--idle-notice-secs",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn poll command");

    thread::sleep(Duration::from_millis(2400));
    child.kill().expect("failed to kill poll command");
    let output = child
        .wait_with_output()
        .expect("failed to collect poll output");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert!(stderr.trim().is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("polling board alpha (id=1) every 1s (idle notice 1s)"));
    assert!(stdout.contains("no update in last 1s"));
}

#[test]
fn board_poll_detects_updates() {
    let env = TestEnv::new();
    env.run_ok(&["init", "--user", "alice"]);
    env.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);

    let mut child = Command::new(env.bin_path())
        .env("HOME", env.home_path())
        .args([
            "board",
            "poll",
            "--user",
            "alice",
            "--board",
            "alpha",
            "--interval",
            "1",
            "--idle-notice-secs",
            "10",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn poll command");

    thread::sleep(Duration::from_millis(1200));
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

    thread::sleep(Duration::from_millis(1800));
    child.kill().expect("failed to kill poll command");
    let output = child
        .wait_with_output()
        .expect("failed to collect poll output");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert!(stderr.trim().is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("update detected:"), "stdout was: {stdout}");
}
