//! Integration tests for `ether-forge groom`.
//!
//! Build a fixture workspace (ROADMAP.md + backlog/ + backlog/done/) inside a
//! tempdir and drive the binary with various flag combinations to exercise:
//!
//! - coverage classification (covered / partial / uncovered / done)
//! - size-vs-sub-step mismatch flagging
//! - stale sub-step path flagging
//! - cascade dry-run vs `--apply` mutation

use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn write(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
}

fn run(workspace: &Path, extra: &[&str]) -> (String, String, bool) {
    let out = Command::new(bin())
        .current_dir(workspace)
        .arg("groom")
        .args(extra)
        .output()
        .expect("run groom");
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.success(),
    )
}

fn setup(workspace: &Path) {
    fs::create_dir_all(workspace.join("backlog/done")).unwrap();
    write(
        &workspace.join("ROADMAP.md"),
        "# Roadmap\n\n\
         ## Phase Alpha\n\n\
         ### World and Entity\n\n\
         World holds entities.\n\n\
         ### Nebula Subsystem\n\n\
         Not touched yet.\n\n\
         ### Component Storage\n\n\
         Sparse sets.\n",
    );
}

#[test]
fn groom_classifies_sections_and_exits_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    setup(ws);

    // Covered: active task whose title/body hit "world" + "entity".
    write(
        &ws.join("backlog/T1-world-entity.md"),
        "---\n\
         id: T1\n\
         title: World and Entity scaffold\n\
         size: M\n\
         status: ready\n\
         ---\n\n\
         ## Sub-steps\n\n\
         - [ ] define world\n\
         - [ ] define entity\n\
         - [ ] wire them up\n",
    );
    // Done: section fully covered by a done task.
    write(
        &ws.join("backlog/done/T2-component-storage.md"),
        "---\n\
         id: T2\n\
         title: Component storage sparse set\n\
         size: S\n\
         status: done\n\
         commit: abc1234\n\
         ---\n\n\
         Body about component storage.\n",
    );

    let (stdout, stderr, ok) = run(ws, &[]);
    assert!(ok, "groom failed: {stderr}");
    assert!(
        stdout.contains("[covered] World and Entity"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("[done] Component Storage"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("[UNCOVERED] Nebula Subsystem"),
        "stdout: {stdout}"
    );
}

#[test]
fn groom_flags_size_mismatch_and_stale_path() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    setup(ws);

    // Size S but 5 sub-steps, and a backticked path that does not exist.
    write(
        &ws.join("backlog/T1-oversize.md"),
        "---\n\
         id: T1\n\
         title: oversize\n\
         size: S\n\
         status: ready\n\
         ---\n\n\
         ## Sub-steps\n\n\
         - [ ] edit `crates/ether-forge/src/ghost.rs`\n\
         - [ ] b\n\
         - [ ] c\n\
         - [ ] d\n\
         - [ ] e\n",
    );

    let (stdout, _stderr, ok) = run(ws, &[]);
    assert!(ok);
    assert!(stdout.contains("size: T1"), "stdout: {stdout}");
    assert!(stdout.contains("stale: T1"), "stdout: {stdout}");
    assert!(stdout.contains("ghost.rs"), "stdout: {stdout}");
}

#[test]
fn groom_cascade_dry_run_does_not_mutate() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    setup(ws);

    write(
        &ws.join("backlog/done/T1-alpha.md"),
        "---\n\
         id: T1\n\
         title: alpha\n\
         size: S\n\
         status: done\n\
         commit: abc1234\n\
         ---\n\n\
         body\n",
    );
    let blocked = "---\n\
         id: T2\n\
         title: beta\n\
         size: S\n\
         status: blocked\n\
         depends_on:\n  - T1\n\
         ---\n\n\
         body\n";
    write(&ws.join("backlog/T2-beta.md"), blocked);

    let (stdout, _stderr, ok) = run(ws, &[]);
    assert!(ok);
    assert!(stdout.contains("T2: drop T1 → ready"), "stdout: {stdout}");

    // Dry run — file must be unchanged.
    let on_disk = fs::read_to_string(ws.join("backlog/T2-beta.md")).unwrap();
    assert_eq!(on_disk, blocked);
}

#[test]
fn groom_cascade_apply_mutates_and_flips_ready() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    setup(ws);

    write(
        &ws.join("backlog/done/T1-alpha.md"),
        "---\n\
         id: T1\n\
         title: alpha\n\
         size: S\n\
         status: done\n\
         commit: abc1234\n\
         ---\n\n\
         body\n",
    );
    write(
        &ws.join("backlog/T2-beta.md"),
        "---\n\
         id: T2\n\
         title: beta\n\
         size: S\n\
         status: blocked\n\
         depends_on:\n  - T1\n\
         ---\n\n\
         body\n",
    );

    let (_stdout, stderr, ok) = run(ws, &["--apply"]);
    assert!(ok, "stderr: {stderr}");

    let on_disk = fs::read_to_string(ws.join("backlog/T2-beta.md")).unwrap();
    assert!(on_disk.contains("status: ready"), "on_disk: {on_disk}");
    assert!(!on_disk.contains("depends_on"), "on_disk: {on_disk}");
    assert!(!on_disk.contains("- T1"), "on_disk: {on_disk}");
}

#[test]
fn groom_json_output_is_valid_json() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    setup(ws);
    write(
        &ws.join("backlog/T1-world-entity.md"),
        "---\n\
         id: T1\n\
         title: World and Entity\n\
         size: S\n\
         status: ready\n\
         ---\n\n\
         ## Sub-steps\n\n\
         - [ ] a\n",
    );

    let (stdout, stderr, ok) = run(ws, &["--json"]);
    assert!(ok, "stderr: {stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(v.get("coverage").is_some());
    assert!(v.get("lint").is_some());
    assert!(v.get("flags").is_some());
    assert!(v.get("cascades").is_some());
}
