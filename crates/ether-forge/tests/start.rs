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
    // Mirror the real repo: ignore the linked worktree root so preflight's
    // dirty check is not tripped by a previous `start` that left a worktree
    // behind (the `--keep-existing` rerun case).
    fs::write(repo.join(".gitignore"), ".claude/worktrees/\n").unwrap();
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

fn run_start_args(repo: &Path, args: &[&str]) -> std::process::Output {
    let mut full = vec!["start"];
    full.extend_from_slice(args);
    Command::new(bin())
        .args(&full)
        .env("ETHER_FORGE_SKIP_CHECK", "1")
        .current_dir(repo)
        .output()
        .unwrap()
}

/// Set up a throwaway repo where the primary worktree has been switched to a
/// non-main feature branch. Simulates the Claude Code on the Web scaffolding
/// case that the in-place fallback targets.
fn setup_repo_on_branch(branch: &str) -> (tempfile::TempDir, PathBuf) {
    let (tmp, repo) = setup_repo("ready");
    run_git(&repo, &["checkout", "-q", "-b", branch]);
    (tmp, repo)
}

/// Set up a repo with a bare remote that has advanced past the local main.
/// Used to prove that `start` fetches origin/main and rebases the new
/// worktree cleanly when main advanced while the session was offline.
fn setup_repo_with_advanced_remote() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let remote = root.join("remote.git");
    let repo = root.join("repo");
    fs::create_dir_all(&remote).unwrap();
    fs::create_dir_all(&repo).unwrap();

    // Bare remote (no working tree, no git config needed beyond init).
    run_git(&remote, &["init", "--bare", "-q", "-b", "main"]);

    // Primary clone.
    run_git(&repo, &["init", "-q", "-b", "main"]);
    run_git(&repo, &["config", "user.email", "t@example.com"]);
    run_git(&repo, &["config", "user.name", "t"]);
    run_git(&repo, &["config", "commit.gpgsign", "false"]);
    run_git(&repo, &["config", "tag.gpgsign", "false"]);
    let backlog = repo.join("backlog");
    fs::create_dir_all(&backlog).unwrap();
    write_task(&backlog, "T40", "start kickoff", "ready");
    fs::write(repo.join("README.md"), "hello\n").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-q", "-m", "init"]);
    run_git(
        &repo,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run_git(&repo, &["push", "-q", "-u", "origin", "main"]);

    // Separate clone advances the remote past our local main.
    let other = root.join("other");
    run_git(
        Path::new(root),
        &["clone", "-q", remote.to_str().unwrap(), "other"],
    );
    run_git(&other, &["config", "user.email", "o@example.com"]);
    run_git(&other, &["config", "user.name", "o"]);
    run_git(&other, &["config", "commit.gpgsign", "false"]);
    run_git(&other, &["config", "tag.gpgsign", "false"]);
    fs::write(other.join("advance.txt"), "advanced\n").unwrap();
    run_git(&other, &["add", "."]);
    run_git(&other, &["commit", "-q", "-m", "advance main"]);
    run_git(&other, &["push", "-q", "origin", "main"]);

    (tmp, repo, remote)
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

    // Final line must be the machine-readable sentinel. Skills grep for
    // `mode=created` to decide whether to follow up with `EnterWorktree`.
    let last = stdout.lines().last().unwrap_or("");
    assert!(
        last.starts_with("start: mode=created "),
        "final line should be the created sentinel: {last}"
    );
    assert!(last.contains("branch=worktree-dev-T40"), "sentinel: {last}");
    assert!(last.contains("path="), "sentinel: {last}");
}

#[test]
fn start_branch_mode_creates_worktree_on_requested_branch() {
    let (_tmp, repo) = setup_repo("ready");

    let out = run_start_args(&repo, &["--branch", "groom-2026-04-14"]);
    assert!(
        out.status.success(),
        "start --branch failed: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let wt = repo.join(".claude/worktrees/groom-2026-04-14");
    assert!(wt.exists(), "worktree dir should exist at {}", wt.display());
    assert!(branch_exists(&repo, "groom-2026-04-14"));

    // Starts at main HEAD.
    assert_eq!(rev_parse(&repo, "main"), rev_parse(&wt, "HEAD"));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("");
    assert!(last.starts_with("start: mode=created "), "sentinel: {last}");
    assert!(last.contains("branch=groom-2026-04-14"), "sentinel: {last}");
}

#[test]
fn start_rebases_when_main_advanced_between_sessions() {
    let (_tmp, repo, _remote) = setup_repo_with_advanced_remote();
    let local_before = rev_parse(&repo, "main");

    let out = run_start(&repo, "T40");
    assert!(
        out.status.success(),
        "start should succeed with advanced remote: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let wt = repo.join(".claude/worktrees/dev-T40");
    // The worktree's HEAD must have advanced past the stale local main so
    // the session starts from the rebased tip.
    let wt_head = rev_parse(&wt, "HEAD");
    assert_ne!(
        wt_head, local_before,
        "worktree HEAD should have advanced onto origin/main"
    );
    let origin_main = rev_parse(&wt, "origin/main");
    assert_eq!(
        wt_head, origin_main,
        "worktree HEAD should match origin/main after rebase"
    );
}

#[test]
fn start_keep_existing_reuses_existing_worktree() {
    let (_tmp, repo) = setup_repo("ready");

    // First run creates the worktree.
    let first = run_start(&repo, "T40");
    assert!(first.status.success(), "first start should succeed");

    // Second run without the flag must error clearly.
    let second = run_start(&repo, "T40");
    assert!(!second.status.success(), "second run should reject");
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("already exists") && stderr.contains("--keep-existing"),
        "stderr should direct user to --keep-existing: {stderr}"
    );

    // With the flag, reuse is OK.
    let third = run_start_args(&repo, &["T40", "--keep-existing"]);
    assert!(
        third.status.success(),
        "--keep-existing should reuse: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&third.stdout),
        String::from_utf8_lossy(&third.stderr)
    );
    let stdout = String::from_utf8_lossy(&third.stdout);
    assert!(
        stdout.contains("reusing existing worktree"),
        "stdout: {stdout}"
    );
    let last = stdout.lines().last().unwrap_or("");
    assert!(last.starts_with("start: mode=created "), "sentinel: {last}");
}

#[test]
fn start_in_place_fallback_task_mode_from_scaffolding_branch() {
    let (_tmp, repo) = setup_repo_on_branch("claude/scaffolding-xyz");

    let out = run_start(&repo, "T40");
    assert!(
        out.status.success(),
        "start should succeed in place: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // No new worktree dir and no new branch.
    assert!(!repo.join(".claude/worktrees/dev-T40").exists());
    assert!(!branch_exists(&repo, "worktree-dev-T40"));

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("already on claude/scaffolding-xyz"),
        "stdout should explain skip: {stdout}"
    );
    let last = stdout.lines().last().unwrap_or("");
    assert_eq!(last, "start: mode=in-place branch=claude/scaffolding-xyz");
}

#[test]
fn start_in_place_fallback_branch_mode_from_scaffolding_branch() {
    let (_tmp, repo) = setup_repo_on_branch("claude/scaffolding-xyz");

    let out = run_start_args(&repo, &["--branch", "groom-2026-04-14"]);
    assert!(
        out.status.success(),
        "start --branch should succeed in place: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // No new worktree or branch — the scaffolding branch is reused.
    assert!(!repo.join(".claude/worktrees/groom-2026-04-14").exists());
    assert!(!branch_exists(&repo, "groom-2026-04-14"));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("");
    assert_eq!(last, "start: mode=in-place branch=claude/scaffolding-xyz");
}

#[test]
fn start_in_place_refuses_when_current_branch_claims_different_task() {
    // Simulate a stale dev-T17 checkout and ask for T40 — must refuse
    // rather than silently succeeding in place and confusing the skill.
    let (_tmp, repo) = setup_repo_on_branch("dev-T17");

    let out = run_start(&repo, "T40");
    assert!(
        !out.status.success(),
        "start should refuse when current branch claims a different task"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not claim T40") && stderr.contains("T17"),
        "stderr should name the conflict: {stderr}"
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
