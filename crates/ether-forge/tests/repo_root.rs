//! Integration test: `ether-forge` resolves `backlog/` via
//! `git rev-parse --show-toplevel`, so invoking it from a nested
//! subdirectory still finds the task files at `<repo_root>/backlog`.

use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "git {args:?} failed in {}",
        repo.display()
    );
}

fn write_task(backlog: &Path, id: &str, title: &str) {
    let body = format!("---\nid: {id}\ntitle: {title}\nsize: S\nstatus: ready\n---\n\n# {title}\n");
    fs::write(backlog.join(format!("{id}-sample.md")), body).unwrap();
}

#[test]
fn list_from_nested_subdirectory_finds_backlog_at_repo_root() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    run_git(repo, &["init", "-q", "-b", "main"]);
    run_git(repo, &["config", "user.email", "t@example.com"]);
    run_git(repo, &["config", "user.name", "t"]);

    let backlog = repo.join("backlog");
    fs::create_dir_all(&backlog).unwrap();
    write_task(&backlog, "T42", "nested lookup");

    let nested = repo.join("crates").join("deep").join("src");
    fs::create_dir_all(&nested).unwrap();

    let out = Command::new(bin())
        .args(["list", "--status", "ready"])
        .current_dir(&nested)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "ether-forge list failed: {}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("T42"),
        "expected T42 in list output, got: {stdout}"
    );
}

#[test]
fn list_outside_git_repo_falls_back_to_cwd_backlog() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path();
    let backlog = cwd.join("backlog");
    fs::create_dir_all(&backlog).unwrap();
    write_task(&backlog, "T7", "cwd fallback");

    let out = Command::new(bin())
        .args(["list", "--status", "ready"])
        .current_dir(cwd)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "ether-forge list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("T7"),
        "expected T7 in list output, got: {stdout}"
    );
}
