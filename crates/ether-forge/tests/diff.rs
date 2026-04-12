//! Integration tests for `ether-forge diff`.
//!
//! Builds a throwaway git repo with a `main` branch containing a `src/lib.rs`
//! and `Cargo.lock`, then switches to a feature branch and modifies both
//! files. Exercises: lockfile filtering, truncation marker, and the
//! non-existent-task-id error path.

use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "t@t.test"]);
    git(dir, &["config", "user.name", "t"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "pub fn old() {}\n").unwrap();
    fs::write(dir.join("Cargo.lock"), "# lock v1\n").unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", "init"]);
}

#[test]
fn diff_strips_lockfile_and_keeps_source() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    fs::write(repo.join("src/lib.rs"), "pub fn new_fn() {}\n").unwrap();
    fs::write(repo.join("Cargo.lock"), "# lock v2 changed\n").unwrap();
    git(repo, &["commit", "-q", "-am", "change"]);

    let out = Command::new(bin())
        .current_dir(repo)
        .arg("diff")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "diff failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("src/lib.rs"),
        "missing src/lib.rs: {stdout}"
    );
    assert!(stdout.contains("new_fn"), "missing change: {stdout}");
    assert!(
        !stdout.contains("Cargo.lock"),
        "lockfile leaked into diff: {stdout}"
    );
}

#[test]
fn diff_truncates_oversized_output() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    // Append a big blob of distinct lines so the diff is well over 200 KB.
    let mut big = String::with_capacity(400_000);
    for i in 0..20_000 {
        big.push_str(&format!("line {i} with some filler text to pad bytes\n"));
    }
    fs::write(repo.join("src/lib.rs"), &big).unwrap();
    git(repo, &["commit", "-q", "-am", "big"]);

    let out = Command::new(bin())
        .current_dir(repo)
        .arg("diff")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("truncated"),
        "expected truncation marker; len={}",
        stdout.len()
    );
    assert!(stdout.len() < 210_000);
}

#[test]
fn diff_errors_on_unknown_task_id() {
    let tmp = tempfile::tempdir().unwrap();
    let backlog = tmp.path().join("backlog");
    fs::create_dir_all(&backlog).unwrap();

    let out = Command::new(bin())
        .current_dir(tmp.path())
        .args(["diff", "T9999", "--backlog-dir"])
        .arg(&backlog)
        .output()
        .unwrap();
    assert!(!out.status.success(), "expected failure on unknown id");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("T9999"),
        "error should mention id: {stderr}"
    );
}
