//! Integration tests for `ether-forge grep`. Exercises the real binary
//! against a fixture tree, then a missing-recipe error path. The "run a real
//! recipe" test skips gracefully when `rg` is not on `PATH` so minimal
//! environments still see the rest of the suite pass.

use std::fs;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn rg_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn write_recipe(rules: &std::path::Path, name: &str, body: &str) {
    fs::create_dir_all(rules).unwrap();
    fs::write(rules.join(format!("{name}.yml")), body).unwrap();
}

#[test]
fn grep_runs_recipe_against_fixture_tree() {
    if !rg_available() {
        eprintln!("skipping: ripgrep not installed");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let rules = tmp.path().join(".claude/rules/grep");
    write_recipe(
        &rules,
        "todo",
        "name: todo\npattern: \"TODO|FIXME\"\npath: src\ndescription: Work markers\n",
    );
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.rs"), "fn a() {} // TODO: implement\n").unwrap();
    fs::write(src.join("b.rs"), "fn b() {} // FIXME: broken\n").unwrap();
    fs::write(src.join("c.rs"), "fn c() {} // clean\n").unwrap();

    let out = Command::new(bin())
        .current_dir(tmp.path())
        .args(["grep", "todo"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "grep failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("TODO: implement"),
        "expected TODO hit: {stdout}"
    );
    assert!(
        stdout.contains("FIXME: broken"),
        "expected FIXME hit: {stdout}"
    );
    assert!(!stdout.contains("c.rs"), "c.rs should not match: {stdout}");

    // Sorted by path — `a.rs` must come before `b.rs` in output.
    let a_pos = stdout.find("a.rs").expect("a.rs in stdout");
    let b_pos = stdout.find("b.rs").expect("b.rs in stdout");
    assert!(a_pos < b_pos, "expected a.rs before b.rs: {stdout}");
}

#[test]
fn grep_reports_missing_recipe_error() {
    let tmp = tempfile::tempdir().unwrap();
    // Intentionally do NOT create any recipe files.
    let out = Command::new(bin())
        .current_dir(tmp.path())
        .args(["grep", "does-not-exist"])
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "grep should fail on missing recipe; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does-not-exist") && stderr.contains("not found"),
        "expected missing-recipe error, got: {stderr}"
    );
}

#[test]
fn grep_list_renders_recipes_sorted() {
    let tmp = tempfile::tempdir().unwrap();
    let rules = tmp.path().join(".claude/rules/grep");
    write_recipe(
        &rules,
        "zebra",
        "name: zebra\npattern: z\ndescription: zed\n",
    );
    write_recipe(
        &rules,
        "apple",
        "name: apple\npattern: a\ndescription: ay\n",
    );

    let out = Command::new(bin())
        .current_dir(tmp.path())
        .args(["grep", "--list"])
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let apple_pos = stdout.find("apple").expect("apple listed");
    let zebra_pos = stdout.find("zebra").expect("zebra listed");
    assert!(apple_pos < zebra_pos, "expected sorted order: {stdout}");
}
