//! Integration tests for `ether-forge merge`.
//!
//! Each test spins up a throwaway git repo with a `main` branch, a T38
//! backlog task, and a linked worktree. The `ETHER_FORGE_SKIP_CHECK=1` env
//! var tells `merge` to skip the verification suite — none of these repos
//! have a Cargo workspace to `cargo clippy` over.

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

fn write_task(backlog: &Path, id: &str, title: &str) {
    let body = format!("---\nid: {id}\ntitle: {title}\nsize: S\nstatus: ready\n---\n\n# {title}\n");
    fs::write(backlog.join(format!("{id}-sample.md")), body).unwrap();
}

/// Set up a git repo with one commit on main, a backlog/ entry for T38, and
/// a linked worktree on branch `dev-T38` with one additional commit.
fn setup_repo_with_worktree() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().to_path_buf();
    run_git(&repo, &["init", "-q", "-b", "main"]);
    run_git(&repo, &["config", "user.email", "t@example.com"]);
    run_git(&repo, &["config", "user.name", "t"]);

    let backlog = repo.join("backlog");
    fs::create_dir_all(&backlog).unwrap();
    write_task(&backlog, "T38", "merge wrap-up");
    fs::write(repo.join("README.md"), "hello\n").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-q", "-m", "init"]);

    let wt = repo.join(".claude/worktrees/dev-T38");
    fs::create_dir_all(wt.parent().unwrap()).unwrap();
    run_git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "dev-T38",
            wt.to_str().unwrap(),
            "main",
        ],
    );
    fs::write(wt.join("feature.txt"), "work\n").unwrap();
    run_git(&wt, &["add", "feature.txt"]);
    run_git(&wt, &["commit", "-q", "-m", "T38: add feature"]);

    (tmp, repo, wt)
}

fn run_merge(repo: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["merge", "T38"];
    args.extend_from_slice(extra);
    Command::new(bin())
        .args(&args)
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

#[test]
fn merge_happy_path_removes_worktree_and_branch() {
    let (_tmp, repo, wt) = setup_repo_with_worktree();

    let out = run_merge(&repo, &[]);
    assert!(
        out.status.success(),
        "merge failed: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(!wt.exists(), "worktree dir should be removed");
    assert!(!branch_exists(&repo, "dev-T38"), "branch should be deleted");

    // main now contains the feature file
    assert!(repo.join("feature.txt").exists());
}

#[test]
fn merge_keep_flag_preserves_worktree_and_branch() {
    let (_tmp, repo, wt) = setup_repo_with_worktree();

    let out = run_merge(&repo, &["--keep"]);
    assert!(
        out.status.success(),
        "merge --keep failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(wt.exists(), "worktree dir should be kept");
    assert!(branch_exists(&repo, "dev-T38"), "branch should be kept");
    assert!(repo.join("feature.txt").exists());
}

#[test]
fn merge_rebases_when_main_advanced_mid_session() {
    let (_tmp, repo, wt) = setup_repo_with_worktree();

    // Advance main with an unrelated file.
    fs::write(repo.join("other.txt"), "upstream\n").unwrap();
    run_git(&repo, &["add", "other.txt"]);
    run_git(&repo, &["commit", "-q", "-m", "upstream change"]);

    let out = run_merge(&repo, &[]);
    assert!(
        out.status.success(),
        "merge after main advance failed: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("rebasing"),
        "expected rebase notice in output: {stdout}"
    );

    assert!(!wt.exists());
    assert!(!branch_exists(&repo, "dev-T38"));
    assert!(repo.join("feature.txt").exists());
    assert!(repo.join("other.txt").exists());
}

#[test]
fn merge_succeeds_when_worktree_dir_was_preremoved() {
    let (_tmp, repo, wt) = setup_repo_with_worktree();

    // Simulate a user that manually `rm -rf`'d the worktree directory
    // after committing. `git worktree list` still lists the entry but the
    // path is gone on disk.
    fs::remove_dir_all(&wt).unwrap();
    assert!(!wt.exists());

    let out = run_merge(&repo, &[]);
    assert!(
        out.status.success(),
        "merge with pre-removed worktree failed: stdout=<{}> stderr=<{}>",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("missing"),
        "expected missing-worktree notice: {stdout}"
    );

    assert!(!branch_exists(&repo, "dev-T38"));
    assert!(repo.join("feature.txt").exists());
}
