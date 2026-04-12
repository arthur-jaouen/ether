//! Integration test: run `ether-forge install-hooks` against a freshly
//! initialized temp repo and assert the hook is written, idempotent, and
//! refuses to clobber a foreign pre-commit hook.

use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn init_repo(path: &Path) {
    fs::create_dir_all(path.join(".git").join("hooks")).unwrap();
}

fn run_install(repo: &Path) -> std::process::Output {
    Command::new(bin())
        .args(["install-hooks", "--repo-root"])
        .arg(repo)
        .output()
        .unwrap()
}

#[test]
fn install_hooks_writes_pre_commit_in_fresh_repo() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());

    let out = run_install(tmp.path());
    assert!(
        out.status.success(),
        "install-hooks failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let hook = tmp.path().join(".git/hooks/pre-commit");
    let body = fs::read_to_string(&hook).unwrap();
    assert!(body.contains("ether-forge:install-hooks"));
    assert!(body.contains("ether-forge check"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&hook).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "hook must be executable");
    }
}

#[test]
fn install_hooks_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());
    assert!(run_install(tmp.path()).status.success());
    let first = fs::read_to_string(tmp.path().join(".git/hooks/pre-commit")).unwrap();
    assert!(run_install(tmp.path()).status.success());
    let second = fs::read_to_string(tmp.path().join(".git/hooks/pre-commit")).unwrap();
    assert_eq!(first, second);
}

#[test]
fn install_hooks_refuses_to_clobber_foreign_hook() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo(tmp.path());
    let hook = tmp.path().join(".git/hooks/pre-commit");
    fs::write(&hook, "#!/bin/sh\necho user hook\n").unwrap();

    let out = run_install(tmp.path());
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not managed by ether-forge"));

    let body = fs::read_to_string(&hook).unwrap();
    assert!(body.contains("user hook"));
}
