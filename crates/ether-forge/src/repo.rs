//! Resolve the workspace root via `git rev-parse --show-toplevel`.
//!
//! Used so every subcommand finds `backlog/`, `ROADMAP.md`, and worktrees
//! relative to the repo root rather than the process cwd. This keeps forge
//! correct when invoked from a nested subdirectory (including worktrees).

use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

/// Return the absolute path to the top of the current git worktree.
///
/// Shells out to `git rev-parse --show-toplevel` from the process cwd.
/// Errors cleanly if the command fails (e.g. not inside a git repo).
pub fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("spawning `git rev-parse --show-toplevel`")?;
    if !output.status.success() {
        return Err(anyhow!(
            "`git rev-parse --show-toplevel` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let raw = String::from_utf8(output.stdout)
        .context("`git rev-parse --show-toplevel` output was not valid UTF-8")?;
    let trimmed = raw.trim_end_matches(['\n', '\r']);
    if trimmed.is_empty() {
        return Err(anyhow!("`git rev-parse --show-toplevel` returned empty"));
    }
    Ok(PathBuf::from(trimmed))
}
