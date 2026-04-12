//! `ether-forge diff [T<n>]` — print a review-scoped `git diff main`.
//!
//! With a task id, runs `git diff main` inside that task's worktree at
//! `worktrees/T<n>-<slug>`. Without an id, runs in the current directory.
//! Lockfile hunks (`Cargo.lock`, `*.lock`) are stripped and oversized output
//! is truncated with a marker so the review surface stays human-sized.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};

use crate::cmd::worktree::{find_task, slugify};

/// Hard cap on bytes of diff output printed to stdout.
const MAX_DIFF_BYTES: usize = 200_000;

/// Run the diff subcommand against the real `git` binary.
pub fn run(backlog_dir: &Path, id: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().context("reading current directory")?;
    let work_dir = match id {
        Some(id) => {
            let task = find_task(backlog_dir, id)?;
            let rel = PathBuf::from("worktrees").join(format!("{}-{}", id, slugify(&task.title)));
            let abs = cwd.join(&rel);
            if !abs.exists() {
                bail!("worktree path does not exist: {}", abs.display());
            }
            abs
        }
        None => cwd,
    };

    let raw = git_diff_main(&work_dir)?;
    let filtered = filter_lockfiles(&raw);
    let capped = truncate(filtered, MAX_DIFF_BYTES);
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(capped.as_bytes())
        .context("writing diff to stdout")?;
    Ok(())
}

/// Capture `git diff main` from `dir` as a UTF-8 string.
fn git_diff_main(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "main"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("spawning `git diff main` in {}", dir.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "`git diff main` failed in {}: {}",
            dir.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).context("git diff output was not valid UTF-8")
}

/// Drop diff sections whose path matches a lockfile pattern.
///
/// Splits on `diff --git` section headers; a section is kept only if neither
/// the `a/` nor the `b/` path is a lockfile.
pub(crate) fn filter_lockfiles(diff: &str) -> String {
    let mut out = String::with_capacity(diff.len());
    let mut section: Option<String> = None;
    let mut keep = true;
    for line in diff.split_inclusive('\n') {
        if line.starts_with("diff --git ") {
            if let Some(prev) = section.take() {
                if keep {
                    out.push_str(&prev);
                }
            }
            keep = !header_is_lockfile(line);
            section = Some(String::from(line));
        } else if let Some(buf) = section.as_mut() {
            buf.push_str(line);
        } else {
            // Preamble before any diff header — always keep.
            out.push_str(line);
        }
    }
    if let Some(prev) = section {
        if keep {
            out.push_str(&prev);
        }
    }
    out
}

/// Return true if a `diff --git a/<p> b/<p>` header names a lockfile path.
fn header_is_lockfile(header: &str) -> bool {
    let tail = match header.strip_prefix("diff --git ") {
        Some(t) => t.trim_end(),
        None => return false,
    };
    tail.split_whitespace().any(|tok| {
        let path = tok.strip_prefix("a/").or_else(|| tok.strip_prefix("b/"));
        match path {
            Some(p) => is_lockfile_path(p),
            None => false,
        }
    })
}

fn is_lockfile_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name == "Cargo.lock" || name.ends_with(".lock")
}

/// Cap `s` to `max` bytes, appending a truncation marker on overflow.
pub(crate) fn truncate(s: String, max: usize) -> String {
    if s.len() <= max {
        return s;
    }
    let dropped = s.len() - max;
    // Find a char boundary at or below `max` so we never split a UTF-8 codepoint.
    let mut cut = max;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = String::with_capacity(cut + 64);
    out.push_str(&s[..cut]);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!(
        "... [truncated {dropped} bytes; full diff exceeds {max}-byte cap]\n"
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_lockfiles_drops_cargo_lock_section() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1..2 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-old
+new
diff --git a/Cargo.lock b/Cargo.lock
index 3..4 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1 +1 @@
-lock-old
+lock-new
";
        let out = filter_lockfiles(diff);
        assert!(out.contains("src/lib.rs"));
        assert!(!out.contains("Cargo.lock"));
        assert!(!out.contains("lock-new"));
    }

    #[test]
    fn filter_lockfiles_drops_nested_dotlock() {
        let diff = "\
diff --git a/vendor/foo.lock b/vendor/foo.lock
@@ -1 +1 @@
-a
+b
diff --git a/README.md b/README.md
@@ -1 +1 @@
-x
+y
";
        let out = filter_lockfiles(diff);
        assert!(!out.contains("foo.lock"));
        assert!(out.contains("README.md"));
    }

    #[test]
    fn filter_lockfiles_keeps_non_lock_dotfiles() {
        let diff = "\
diff --git a/crates/a/src/clock.rs b/crates/a/src/clock.rs
@@ -1 +1 @@
-a
+b
";
        let out = filter_lockfiles(diff);
        assert!(out.contains("clock.rs"));
    }

    #[test]
    fn truncate_below_cap_is_noop() {
        let s = "hello\nworld\n".to_string();
        let len = s.len();
        let out = truncate(s, 1024);
        assert_eq!(out.len(), len);
    }

    #[test]
    fn truncate_appends_marker_when_over_cap() {
        let s = "a".repeat(500);
        let out = truncate(s, 100);
        assert!(out.starts_with(&"a".repeat(100)));
        assert!(out.contains("truncated 400 bytes"));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        // Two 2-byte chars; cap in the middle of the second char.
        let s = "éé".to_string();
        assert_eq!(s.len(), 4);
        let out = truncate(s, 3);
        // Must not panic and must not include the partially-cut char.
        assert!(out.starts_with('é'));
        assert!(out.contains("truncated"));
    }

    #[test]
    fn header_is_lockfile_matches_cargo_lock() {
        assert!(header_is_lockfile("diff --git a/Cargo.lock b/Cargo.lock\n"));
        assert!(header_is_lockfile(
            "diff --git a/sub/foo.lock b/sub/foo.lock\n"
        ));
        assert!(!header_is_lockfile(
            "diff --git a/src/main.rs b/src/main.rs\n"
        ));
    }
}
