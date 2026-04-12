//! Integration test: run `ether-forge find` against a fixture Rust file and
//! assert the expected match shows up in stdout. Skipped gracefully when
//! `ast-grep` is not on `PATH` so the rest of the workspace test suite still
//! runs in minimal environments.

use std::fs;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ether-forge")
}

fn ast_grep_available() -> bool {
    Command::new("ast-grep")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn find_pattern_matches_fixture_file() {
    if !ast_grep_available() {
        eprintln!("skipping: ast-grep not installed");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let fixture = tmp.path().join("fixture.rs");
    fs::write(
        &fixture,
        "fn main() {\n    let x: Option<i32> = Some(1);\n    let _ = x.unwrap();\n}\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["find", "$X.unwrap()", "--path"])
        .arg(&fixture)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "find failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("x.unwrap()"),
        "expected match in stdout, got: {stdout}"
    );
}

#[test]
fn find_rule_resolves_from_rules_dir() {
    if !ast_grep_available() {
        eprintln!("skipping: ast-grep not installed");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let rules = tmp.path().join(".claude/rules/sg");
    fs::create_dir_all(&rules).unwrap();
    fs::write(
        rules.join("local-unwrap.yml"),
        "id: local-unwrap\nmessage: test\nseverity: warning\nlanguage: rust\nrule:\n  pattern: $X.unwrap()\n",
    )
    .unwrap();
    let fixture = tmp.path().join("fixture.rs");
    fs::write(
        &fixture,
        "fn main() { let x: Option<i32> = Some(1); let _ = x.unwrap(); }\n",
    )
    .unwrap();

    // ast-grep scan expects to find the rule file relative to CWD.
    let out = Command::new(bin())
        .current_dir(tmp.path())
        .args(["find", "--rule", "local-unwrap", "--path", "fixture.rs"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "find --rule failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("x.unwrap()") || stdout.contains("local-unwrap"),
        "expected match in stdout, got: {stdout}"
    );
}
