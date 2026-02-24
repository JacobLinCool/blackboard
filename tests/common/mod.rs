#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::assert::OutputAssertExt;
use tempfile::TempDir;

pub struct TestEnv {
    home: TempDir,
    bin: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        Self {
            home: tempfile::tempdir().expect("failed to create temp HOME"),
            bin: PathBuf::from(assert_cmd::cargo::cargo_bin!("blackboard")),
        }
    }

    pub fn db_path(&self) -> PathBuf {
        self.home.path().join(".blackboard").join("blackboard.db")
    }

    pub fn run_ok(&self, args: &[&str]) -> String {
        let mut cmd = self.command(args);
        let assert = cmd.assert().success();
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf8")
    }

    pub fn run_err(&self, args: &[&str]) -> String {
        let mut cmd = self.command(args);
        let assert = cmd.assert().failure();
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf8")
    }

    pub fn run_fail_output(&self, args: &[&str]) -> (String, String) {
        let mut cmd = self.command(args);
        let assert = cmd.assert().failure();
        let stdout =
            String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf8");
        let stderr =
            String::from_utf8(assert.get_output().stderr.clone()).expect("stderr should be utf8");
        (stdout, stderr)
    }

    pub fn bin_path(&self) -> &Path {
        &self.bin
    }

    pub fn home_path(&self) -> &Path {
        self.home.path()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(&self.bin);
        cmd.env("HOME", self.home.path());
        cmd.args(args);
        cmd
    }
}
