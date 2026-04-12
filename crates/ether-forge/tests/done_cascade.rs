//! Integration test: run the `done` subcommand against a fixture backlog
//! with chained dependencies and assert the cascade semantics.

use std::fs;
use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn write(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn done_completes_task_and_cascades_dependencies() {
    let tmp = tempfile::tempdir().unwrap();
    let backlog = tmp.path().join("backlog");
    fs::create_dir_all(backlog.join("done")).unwrap();

    // T1 is ready and about to be completed.
    write(
        &backlog.join("T1-foo.md"),
        "---\n\
         id: T1\n\
         title: foo\n\
         size: S\n\
         status: ready\n\
         priority: 1\n\
         ---\n\
         \n\
         ## Sub-steps\n\
         \n\
         - [ ] do the thing\n",
    );

    // T2 depends only on T1 → should flip to ready after cascade.
    write(
        &backlog.join("T2-bar.md"),
        "---\n\
         id: T2\n\
         title: bar\n\
         size: S\n\
         status: blocked\n\
         depends_on:\n  \
         - T1\n\
         priority: 2\n\
         ---\n\
         \n\
         body kept verbatim\n",
    );

    // T3 depends on T1 and T2 → should stay blocked, only T1 removed.
    write(
        &backlog.join("T3-baz.md"),
        "---\n\
         id: T3\n\
         title: baz\n\
         size: M\n\
         status: blocked\n\
         depends_on:\n  \
         - T1\n  \
         - T2\n\
         ---\n\
         \n\
         body three\n",
    );

    // T4 does not depend on T1 → untouched.
    let t4_raw = "---\n\
         id: T4\n\
         title: qux\n\
         size: S\n\
         status: ready\n\
         ---\n\
         \n\
         body four\n";
    write(&backlog.join("T4-qux.md"), t4_raw);

    let status = Command::new(bin())
        .args(["done", "T1", "--commit", "abc1234", "--backlog-dir"])
        .arg(&backlog)
        .status()
        .unwrap();
    assert!(status.success());

    // T1 moved to done/ with commit + status=done, sub-steps stripped.
    assert!(!backlog.join("T1-foo.md").exists());
    let t1_done = read(&backlog.join("done/T1-foo.md"));
    assert!(t1_done.contains("status: done"));
    assert!(t1_done.contains("commit: abc1234"));
    assert!(!t1_done.contains("Sub-steps"));
    assert!(!t1_done.contains("do the thing"));

    // T2: depends_on empty → field removed, status flipped.
    let t2 = read(&backlog.join("T2-bar.md"));
    assert!(t2.contains("status: ready"));
    assert!(!t2.contains("depends_on"));
    assert!(!t2.contains("- T1"));
    assert!(t2.contains("priority: 2"));
    assert!(t2.contains("body kept verbatim"));

    // T3: still blocked, T1 removed, T2 dep preserved.
    let t3 = read(&backlog.join("T3-baz.md"));
    assert!(t3.contains("status: blocked"));
    assert!(t3.contains("depends_on:"));
    assert!(!t3.contains("- T1"));
    assert!(t3.contains("- T2"));
    assert!(t3.contains("body three"));

    // T4: byte-for-byte untouched.
    assert_eq!(read(&backlog.join("T4-qux.md")), t4_raw);
}

#[test]
fn done_refuses_when_already_done() {
    let tmp = tempfile::tempdir().unwrap();
    let backlog = tmp.path().join("backlog");
    fs::create_dir_all(backlog.join("done")).unwrap();

    write(
        &backlog.join("T1-foo.md"),
        "---\nid: T1\ntitle: foo\nsize: S\nstatus: done\npriority: 1\ncommit: deadbeef\n---\n\n",
    );

    let out = Command::new(bin())
        .args(["done", "T1", "--backlog-dir"])
        .arg(&backlog)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("already done"), "stderr: {stderr}");
}

#[test]
fn done_refuses_when_dependencies_unsatisfied() {
    let tmp = tempfile::tempdir().unwrap();
    let backlog = tmp.path().join("backlog");
    fs::create_dir_all(backlog.join("done")).unwrap();

    write(
        &backlog.join("T5-blocked.md"),
        "---\n\
         id: T5\n\
         title: blocked task\n\
         size: S\n\
         status: blocked\n\
         depends_on:\n  \
         - T1\n\
         ---\n\
         \n",
    );

    let out = Command::new(bin())
        .args(["done", "T5", "--backlog-dir"])
        .arg(&backlog)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unsatisfied"), "stderr: {stderr}");
}
