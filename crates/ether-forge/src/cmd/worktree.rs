//! `ether-forge worktree T<n>` — create a git worktree + branch for a task.
//!
//! Derives a slug from the task title, creates
//! `worktrees/T<n>-<slug>` on branch `task/T<n>` based on `main`, and prints
//! the absolute path of the new worktree. Refuses if the worktree directory
//! or branch already exists.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{anyhow, bail, Context, Result};

use crate::repo;
use crate::task::Task;

/// Spawn the real `git` binary and return the resulting exit status.
pub(crate) fn spawn_real(argv: &[&str]) -> Result<ExitStatus> {
    let (program, args) = argv.split_first().ok_or_else(|| anyhow!("empty command"))?;
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("spawning `{}`", argv.join(" ")))?;
    Ok(status)
}

/// Run the worktree command against the real filesystem and `git` binary.
pub fn run(backlog_dir: &Path, id: &str) -> Result<()> {
    let task = find_task(backlog_dir, id)?;
    let slug = slugify(&task.title);
    let repo_root = repo::repo_root()?;
    let worktree_rel = PathBuf::from("worktrees").join(format!("{id}-{slug}"));
    let worktree_abs = repo_root.join(&worktree_rel);
    let branch = format!("task/{id}");

    if worktree_abs.exists() {
        bail!("worktree path already exists: {}", worktree_abs.display());
    }
    if branch_exists(&mut spawn_real, &branch)? {
        bail!("branch already exists: {branch}");
    }

    let worktree_arg = worktree_rel
        .to_str()
        .ok_or_else(|| anyhow!("non-UTF8 worktree path"))?;
    let argv = vec![
        "git",
        "worktree",
        "add",
        worktree_arg,
        "-b",
        &branch,
        "main",
    ];
    let status = spawn_real(&argv)?;
    if !status.success() {
        bail!("`{}` failed with {}", argv.join(" "), status);
    }

    println!("{}", worktree_abs.display());
    Ok(())
}

/// Check whether `branch` currently exists in the local repo.
pub(crate) fn branch_exists<R>(runner: &mut R, branch: &str) -> Result<bool>
where
    R: FnMut(&[&str]) -> Result<ExitStatus>,
{
    let argv = ["git", "rev-parse", "--verify", "--quiet", branch];
    let status = runner(&argv)?;
    Ok(status.success())
}

/// Look up a task file by id, erroring if zero or multiple match.
pub(crate) fn find_task(backlog_dir: &Path, id: &str) -> Result<Task> {
    let tasks = Task::load_all(backlog_dir)?;
    let matches: Vec<Task> = tasks.into_iter().filter(|t| t.id == id).collect();
    match matches.len() {
        0 => Err(anyhow!(
            "no task found with id {id} in {}",
            backlog_dir.display()
        )),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(anyhow!("multiple tasks matched id {id}")),
    }
}

/// Derive a filesystem-safe slug from a task title.
///
/// Lowercased, non-alphanumeric runs collapse to a single `-`, leading and
/// trailing dashes are trimmed. Empty titles fall back to `task`.
pub(crate) fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = true;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "task".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("World and Entity types"), "world-and-entity-types");
    }

    #[test]
    fn slugify_collapses_punctuation() {
        assert_eq!(
            slugify("ether-forge worktree & commit!!"),
            "ether-forge-worktree-commit"
        );
    }

    #[test]
    fn slugify_handles_unicode_and_empty() {
        assert_eq!(slugify("---"), "task");
        assert_eq!(slugify("café au lait"), "caf-au-lait");
    }

    #[test]
    fn branch_exists_true_when_rev_parse_succeeds() {
        let mut runner = |argv: &[&str]| {
            assert_eq!(argv[0..4], ["git", "rev-parse", "--verify", "--quiet"]);
            Ok(ExitStatus::from_raw(0))
        };
        assert!(branch_exists(&mut runner, "task/T9").unwrap());
    }

    #[test]
    fn branch_exists_false_when_rev_parse_fails() {
        let mut runner = |_argv: &[&str]| Ok(ExitStatus::from_raw(1 << 8));
        assert!(!branch_exists(&mut runner, "task/T9").unwrap());
    }
}
