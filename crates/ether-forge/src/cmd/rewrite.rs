//! `ether-forge rewrite` — thin wrapper around `ast-grep`'s rewrite mode.
//!
//! `rewrite <pattern> --to <replacement> [--lang rust] [path]` shells out to
//! `ast-grep run -p <pattern> --rewrite <replacement> --lang <lang> -U
//! [path]`. The `-U` flag applies edits in place; without it ast-grep only
//! prints a diff preview.
//!
//! Requires `ast-grep` on `PATH` (`cargo install ast-grep`).

use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{bail, Context, Result};

/// Assemble the `ast-grep` argv for a `rewrite` invocation. Exposed for tests.
pub fn build_argv(
    pattern: &str,
    replacement: &str,
    lang: &str,
    path: Option<&Path>,
) -> Vec<String> {
    let mut argv = vec![
        "ast-grep".to_string(),
        "run".to_string(),
        "-p".to_string(),
        pattern.to_string(),
        "--rewrite".to_string(),
        replacement.to_string(),
        "--lang".to_string(),
        lang.to_string(),
        "-U".to_string(),
    ];
    if let Some(path) = path {
        argv.push(path.display().to_string());
    }
    argv
}

/// Run `ether-forge rewrite` with the real `ast-grep` binary.
pub fn run(pattern: &str, replacement: &str, lang: &str, path: Option<&Path>) -> Result<()> {
    let argv = build_argv(pattern, replacement, lang, path);
    let status = spawn(&argv)?;
    if !status.success() {
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

    #[test]
    fn argv_contains_rewrite_and_update_flags() {
        let argv = build_argv("$X.unwrap()", "$X.expect(\"todo\")", "rust", None);
        assert_eq!(argv[0..2], ["ast-grep", "run"]);
        assert!(argv.windows(2).any(|w| w == ["-p", "$X.unwrap()"]));
        assert!(argv
            .windows(2)
            .any(|w| w == ["--rewrite", "$X.expect(\"todo\")"]));
        assert!(argv.windows(2).any(|w| w == ["--lang", "rust"]));
        assert!(argv.contains(&"-U".to_string()));
    }

    #[test]
    fn argv_appends_path_when_given() {
        let argv = build_argv("foo", "bar", "rust", Some(Path::new("crates/ether-core")));
        assert_eq!(argv.last().unwrap(), "crates/ether-core");
    }

    #[test]
    fn argv_honors_custom_lang() {
        let argv = build_argv("foo", "bar", "python", None);
        assert!(argv.windows(2).any(|w| w == ["--lang", "python"]));
    }
}
