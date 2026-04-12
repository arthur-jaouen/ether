//! `ether-forge find` — thin wrapper around `ast-grep` for structural search.
//!
//! Two modes:
//!
//! - **Pattern**: `find <pattern> [--lang rust] [path]` runs
//!   `ast-grep run -p <pattern> --lang <lang> [path]`.
//! - **Rule**: `find --rule <name> [path]` resolves
//!   `.claude/rules/sg/<name>.yml` relative to the current working directory
//!   and runs `ast-grep scan -r <file> [path]`.
//!
//! Requires `ast-grep` on `PATH` (`cargo install ast-grep`).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{bail, Context, Result};

/// Directory (relative to CWD) where structural-search rule files live.
pub const RULES_DIR: &str = ".claude/rules/sg";

/// Resolve a rule name to an absolute-ish path under [`RULES_DIR`]. Errors if
/// the file does not exist.
pub fn resolve_rule(name: &str) -> Result<PathBuf> {
    let path = PathBuf::from(RULES_DIR).join(format!("{name}.yml"));
    if !path.exists() {
        bail!("rule `{}` not found at {}", name, path.display());
    }
    Ok(path)
}

/// Assemble the `ast-grep` argv for a `find` invocation. Exposed for tests.
///
/// One of `pattern` or `rule` must be provided; supplying both, or neither,
/// is an error.
pub fn build_argv(
    pattern: Option<&str>,
    lang: &str,
    rule: Option<&str>,
    path: Option<&Path>,
) -> Result<Vec<String>> {
    match (pattern, rule) {
        (None, None) => bail!("find: provide a pattern or --rule"),
        (Some(_), Some(_)) => bail!("find: pattern and --rule are mutually exclusive"),
        (Some(p), None) => {
            let mut argv = vec![
                "ast-grep".to_string(),
                "run".to_string(),
                "-p".to_string(),
                p.to_string(),
                "--lang".to_string(),
                lang.to_string(),
            ];
            if let Some(path) = path {
                argv.push(path.display().to_string());
            }
            Ok(argv)
        }
        (None, Some(r)) => {
            let rule_path = resolve_rule(r)?;
            let mut argv = vec![
                "ast-grep".to_string(),
                "scan".to_string(),
                "-r".to_string(),
                rule_path.display().to_string(),
            ];
            if let Some(path) = path {
                argv.push(path.display().to_string());
            }
            Ok(argv)
        }
    }
}

/// Run `ether-forge find` with the real `ast-grep` binary.
pub fn run(
    pattern: Option<&str>,
    lang: &str,
    rule: Option<&str>,
    path: Option<&Path>,
) -> Result<()> {
    let argv = build_argv(pattern, lang, rule, path)?;
    let status = spawn(&argv)?;
    if !status.success() {
        // ast-grep exits non-zero when there are no matches; surface that as
        // a clean error rather than a panic.
        bail!("`{}` exited with {}", argv.join(" "), status);
    }
    Ok(())
}

fn spawn(argv: &[String]) -> Result<ExitStatus> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("empty command"))?;
    Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("spawning `{}` (is ast-grep installed?)", argv.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn pattern_argv_defaults_to_rust_lang() {
        let argv = build_argv(Some("$X.unwrap()"), "rust", None, None).unwrap();
        assert_eq!(argv[0], "ast-grep");
        assert_eq!(argv[1], "run");
        assert!(argv.windows(2).any(|w| w == ["-p", "$X.unwrap()"]));
        assert!(argv.windows(2).any(|w| w == ["--lang", "rust"]));
    }

    #[test]
    fn pattern_argv_appends_path_when_given() {
        let argv = build_argv(
            Some("foo"),
            "rust",
            None,
            Some(Path::new("crates/ether-core")),
        )
        .unwrap();
        assert_eq!(argv.last().unwrap(), "crates/ether-core");
    }

    #[test]
    fn pattern_argv_honors_custom_lang() {
        let argv = build_argv(Some("foo"), "python", None, None).unwrap();
        assert!(argv.windows(2).any(|w| w == ["--lang", "python"]));
    }

    #[test]
    fn rule_argv_uses_scan_with_resolved_path() {
        let tmp = tempfile::tempdir().unwrap();
        let rules = tmp.path().join(RULES_DIR);
        fs::create_dir_all(&rules).unwrap();
        fs::write(rules.join("demo.yml"), "id: demo\n").unwrap();

        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let argv = build_argv(None, "rust", Some("demo"), None).unwrap();
        std::env::set_current_dir(cwd).unwrap();

        assert_eq!(argv[0..3], ["ast-grep", "scan", "-r"]);
        assert!(argv[3].ends_with("demo.yml"));
    }

    #[test]
    fn rule_argv_errors_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let err = build_argv(None, "rust", Some("nope"), None).unwrap_err();
        std::env::set_current_dir(cwd).unwrap();
        assert!(format!("{err:#}").contains("rule `nope` not found"));
    }

    #[test]
    fn requires_pattern_or_rule() {
        let err = build_argv(None, "rust", None, None).unwrap_err();
        assert!(format!("{err:#}").contains("pattern or --rule"));
    }

    #[test]
    fn rejects_both_pattern_and_rule() {
        let err = build_argv(Some("x"), "rust", Some("y"), None).unwrap_err();
        assert!(format!("{err:#}").contains("mutually exclusive"));
    }
}
