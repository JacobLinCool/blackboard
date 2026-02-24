use std::hint::black_box;
use std::path::PathBuf;
use std::process::Command;

use criterion::{Criterion, criterion_group, criterion_main};
use tempfile::TempDir;

struct BenchEnv {
    home: TempDir,
    bin: PathBuf,
}

impl BenchEnv {
    fn new() -> Self {
        Self {
            home: tempfile::tempdir().expect("failed to create temp HOME"),
            bin: PathBuf::from(assert_cmd::cargo::cargo_bin!("blackboard")),
        }
    }

    fn bootstrap_owner(&self) {
        self.run_ok(&["init", "--user", "alice"]);
        self.run_ok(&["board", "create", "--user", "alice", "--name", "alpha"]);
    }

    fn run_ok(&self, args: &[&str]) -> String {
        let output = Command::new(&self.bin)
            .env("HOME", self.home.path())
            .args(args)
            .output()
            .expect("failed to run blackboard command");

        if !output.status.success() {
            panic!(
                "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
                args,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        String::from_utf8(output.stdout).expect("stdout should be utf8")
    }

    fn run_ok_owned(&self, args: Vec<String>) -> String {
        let output = Command::new(&self.bin)
            .env("HOME", self.home.path())
            .args(args.iter().map(String::as_str))
            .output()
            .expect("failed to run blackboard command");

        if !output.status.success() {
            panic!(
                "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
                args,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        String::from_utf8(output.stdout).expect("stdout should be utf8")
    }
}

fn bench_task_add(c: &mut Criterion) {
    let env = BenchEnv::new();
    env.bootstrap_owner();

    let mut index = 0usize;
    c.bench_function("cli/task_add_owner", |b| {
        b.iter(|| {
            index += 1;
            let title = format!("task-{index}");
            let out = env.run_ok_owned(vec![
                "task".to_string(),
                "add".to_string(),
                "--user".to_string(),
                "alice".to_string(),
                "--board".to_string(),
                "alpha".to_string(),
                "--title".to_string(),
                title,
                "--description".to_string(),
                "benchmark".to_string(),
            ]);
            black_box(out);
        })
    });
}

fn bench_task_list(c: &mut Criterion) {
    let env = BenchEnv::new();
    env.bootstrap_owner();

    for i in 0..500 {
        let title = format!("seed-{i}");
        env.run_ok_owned(vec![
            "task".to_string(),
            "add".to_string(),
            "--user".to_string(),
            "alice".to_string(),
            "--board".to_string(),
            "alpha".to_string(),
            "--title".to_string(),
            title,
            "--description".to_string(),
            "seed".to_string(),
        ]);
    }

    c.bench_function("cli/task_list_500", |b| {
        b.iter(|| {
            let out = env.run_ok(&["task", "list", "--user", "alice", "--board", "alpha"]);
            black_box(out);
        })
    });
}

fn bench_dependency_edit(c: &mut Criterion) {
    let env = BenchEnv::new();
    env.bootstrap_owner();

    for i in 0..120 {
        let title = format!("dep-seed-{i}");
        env.run_ok_owned(vec![
            "task".to_string(),
            "add".to_string(),
            "--user".to_string(),
            "alice".to_string(),
            "--board".to_string(),
            "alpha".to_string(),
            "--title".to_string(),
            title,
            "--description".to_string(),
            "seed".to_string(),
        ]);
    }

    let mut toggle = false;
    c.bench_function("cli/task_edit_dependency", |b| {
        b.iter(|| {
            toggle = !toggle;
            let depends = if toggle { "1,2,3" } else { "4,5,6" };
            let out = env.run_ok_owned(vec![
                "task".to_string(),
                "edit".to_string(),
                "--user".to_string(),
                "alice".to_string(),
                "--board".to_string(),
                "alpha".to_string(),
                "--task-id".to_string(),
                "120".to_string(),
                "--depends-on".to_string(),
                depends.to_string(),
            ]);
            black_box(out);
        })
    });
}

criterion_group!(
    benches,
    bench_task_add,
    bench_task_list,
    bench_dependency_edit
);
criterion_main!(benches);
