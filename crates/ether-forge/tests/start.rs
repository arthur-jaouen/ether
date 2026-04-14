//! Integration tests for `ether-forge start`.
//!
//! Spins up a throwaway git repo with a `main` branch and a `T40` backlog
//! task, drives `ether-forge start T40` against it, and asserts the worktree
//! directory + branch were created and the worktree HEAD matches main. The
//! `ETHER_FORGE_SKIP_CHECK=1` env var bypasses the cargo verification suite —
//! none of these repos are Cargo workspaces.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn run_git(repo: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?} failed in {}: {}",
        repo.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_task(backlog: &Path, id: &str, title: &str, status: &str) {
    let body =
        format!("---\nid: {id}\ntitle: {title}\nsize: S\nstatus: {status}\n---\n\n# {title}\n");
    fs::write(backlog.join(format!("{id}-sample.md")), body).unwrap();
}

/// Set up a git repo with one commit on main and a backlog/ entry for T40.
fn setup_repo(status: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().to_path_buf();
    run_git(&repo, &["init", "-q", "-b", "main"]);
    run_git(&repo, &["config", "user.email", "t@example.com"]);
    run_git(&repo, &["config", "user.name", "t"]);
    run_git(&repo, &["config", "commit.gpgsign", "false"]);
    run_git(&repo, &["config", "tag.gpgsign", "false"]);

    let backlog = repo.join("backlog");
    fs::create_dir_all(&backlog).unwrap();
    write_task(&backlog, "T40", "start kickoff", status);
    fs::write(repo.join("README.md"), "hello\n").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-q", "-m", "init"]);

    (tmp, repo)
}

fn run_start(repo: &Path, id: &str) -> std::process::Output {
    Command::new(bin())
        .args(["start", id])
        .env("ETHER_FORGE_SKIP_CHECK", "1")
        .current_dir(repo)
        .output()
        .unwrap()
}

fn branch_exists(repo: &Path, name: &str) -> bool {
    let out = Command::new("git")
        .args(["branch", "--list", name])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim() != ""
}

fn rev_parse(repo: &Path, refname: &str) -> String {
    let out = Command::new("git")
        .args(["rev-parse", refname])
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn start_happy_path_creates_worktree_and_branch() {
    let (_tmp, repo) = setup_repo("ready");

    let out = run_start(&repo, "T40");
    assert!(
        out.status.success(),
        "start failed: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let wt = repo.join(".claude/worktrees/dev-T40");
    assert!(
        wt.exists(),
        "worktree dir should be created at {}",
        wt.display()
    );
    assert!(branch_exists(&repo, "worktree-dev-T40"));

    // The new worktree should start at the same commit as main.
    let main_head = rev_parse(&repo, "main");
    let wt_head = rev_parse(&wt, "HEAD");
    assert_eq!(main_head, wt_head);

    // Output advertises the worktree path and branch.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dev-T40"), "stdout missing path: {stdout}");
    assert!(
        stdout.contains("worktree-dev-T40"),
        "stdout missing branch: {stdout}"
    );
}

#[test]
fn start_refuses_when_task_not_ready() {
    let (_tmp, repo) = setup_repo("draft");

    let out = run_start(&repo, "T40");
    assert!(!out.status.success(), "start should refuse draft task");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not ready") || stderr.contains("draft"),
        "stderr should explain: {stderr}"
    );

    assert!(!repo.join(".claude/worktrees/dev-T40").exists());
    assert!(!branch_exists(&repo, "worktree-dev-T40"));
}

#[test]
fn start_refuses_when_task_branch_already_claimed() {
    let (_tmp, repo) = setup_repo("ready");

    // Pre-create a branch that claims T40 — preflight should reject.
    run_git(&repo, &["branch", "dev-T40"]);

    let out = run_start(&repo, "T40");
    assert!(
        !out.status.success(),
        "start should refuse pre-claimed task"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("claims T40"),
        "stderr should mention claim: {stderr}"
    );

    assert!(!repo.join(".claude/worktrees/dev-T40").exists());
    assert!(!branch_exists(&repo, "worktree-dev-T40"));
}
