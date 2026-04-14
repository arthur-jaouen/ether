//! `ether-forge start T<n>` — collapse the `/dev` kickoff dance into one
//! primitive. Loads the task, asserts it is `ready`, reuses `check` and
//! `preflight` in-process, creates a linked worktree at
//! `.claude/worktrees/dev-T<n>` on branch `worktree-dev-T<n>`, then fetches
//! `main` and rebases the worktree if it is behind. The entry-side mirror of
//! [`crate::cmd::merge`].
//!
//! Pure helpers (`is_behind_main`, `worktree_exists`) are unit-tested without
//! spawning git; the top-level `run` orchestrates real git + `check` calls.
//!
//! Tests set `ETHER_FORGE_SKIP_CHECK=1` (the same seam used by `merge`) to
//! bypass the verification suite when driving `ether-forge start` against a
//! throwaway repo that has no Cargo workspace. Production callers never set
//! this — the variable is an internal test seam.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::cmd::check;
use crate::cmd::merge::{parse_worktree_list, WorktreeEntry, SKIP_CHECK_ENV};
use crate::cmd::preflight;
use crate::task::{find_task, Status};

/// `main` is behind `origin/main` iff their trimmed revs differ.
pub(crate) fn is_behind_main(local_main: &str, origin_main: &str) -> bool {
    !origin_main.trim().is_empty() && local_main.trim() != origin_main.trim()
}

/// Does any registered worktree entry live at `target`?
pub(crate) fn worktree_exists(entries: &[WorktreeEntry], target: &Path) -> bool {
    entries.iter().any(|e| e.path == target)
}

/// Build the worktree path for task `id`, rooted at the main worktree.
pub(crate) fn worktree_path_for(main_path: &Path, id: &str) -> PathBuf {
    main_path
        .join(".claude/worktrees")
        .join(format!("dev-{id}"))
}

/// Build the branch name for task `id`.
pub(crate) fn branch_name_for(id: &str) -> String {
    format!("worktree-dev-{id}")
}

/// Run the start subcommand against the real filesystem and `git` binary.
pub fn run(backlog_dir: &Path, id: &str) -> Result<()> {
    let task = find_task(backlog_dir, id)?;
    if task.status != Status::Ready {
        bail!("task {id} is not ready (status: {})", task.status.as_str());
    }

    preflight::run(backlog_dir, Some(id)).context("preflight failed — refusing to start")?;

    if std::env::var_os(SKIP_CHECK_ENV).is_none() {
        check::run().context("ether-forge check failed — start aborted")?;
    }

    let cwd = std::env::current_dir().context("reading current directory")?;
    let list_raw = git_output(&cwd, &["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&list_raw);
    let main_path = entries
        .first()
        .map(|e| e.path.clone())
        .ok_or_else(|| anyhow::anyhow!("no git worktrees listed"))?;

    let wt_path = worktree_path_for(&main_path, id);
    if worktree_exists(&entries, &wt_path) {
        bail!("worktree {} already exists", wt_path.display());
    }

    let branch = branch_name_for(id);
    if let Some(parent) = wt_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating worktree parent dir {}", parent.display()))?;
    }

    let wt_str = wt_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("worktree path is not UTF-8"))?;
    let add = Command::new("git")
        .current_dir(&main_path)
        .args(["worktree", "add", "-b", &branch, wt_str, "main"])
        .status()
        .context("spawning git worktree add")?;
    if !add.success() {
        bail!("`git worktree add` failed for {}", wt_path.display());
    }

    // Fetch and rebase from inside the new worktree. We skip the fetch when
    // there is no `origin` remote (throwaway test repos, fully-local setups);
    // a missing remote is not an error here. After fetching we compare local
    // `main` against `origin/main` to decide whether to rebase.
    let has_origin = Command::new("git")
        .current_dir(&wt_path)
        .args(["remote", "get-url", "origin"])
        .output()
        .context("spawning git remote get-url")?
        .status
        .success();

    if has_origin {
        let fetch = Command::new("git")
            .current_dir(&wt_path)
            .args(["fetch", "origin", "main"])
            .status()
            .context("spawning git fetch")?;
        if !fetch.success() {
            bail!("`git fetch origin main` failed in {}", wt_path.display());
        }
    }

    let local_main = git_output(&wt_path, &["rev-parse", "main"])?;
    // `origin/main` may not exist (no remote, or fetch was skipped) — treat
    // absence as "not behind" instead of an error.
    let origin_main = git_output(&wt_path, &["rev-parse", "origin/main"]).unwrap_or_default();
    if is_behind_main(&local_main, &origin_main) {
        println!("start: rebasing {branch} onto origin/main");
        let rebase = Command::new("git")
            .current_dir(&wt_path)
            .args(["rebase", "origin/main"])
            .status()
            .context("spawning git rebase")?;
        if !rebase.success() {
            bail!(
                "`git rebase origin/main` failed in {} — resolve conflicts and re-run",
                wt_path.display()
            );
        }
    }

    println!("start: created worktree {}", wt_path.display());
    println!("start: on branch {branch}");
    println!("start: next → cd {} && implement {id}", wt_path.display());
    Ok(())
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .with_context(|| format!("spawning `git {}`", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_behind_main_detects_divergence() {
        assert!(is_behind_main("aaa111", "bbb222"));
        assert!(is_behind_main("aaa111\n", "bbb222\n"));
    }

    #[test]
    fn is_behind_main_false_when_equal() {
        assert!(!is_behind_main("abc1234", "abc1234"));
        assert!(!is_behind_main("abc1234\n", "abc1234"));
    }

    #[test]
    fn is_behind_main_treats_missing_origin_as_not_behind() {
        // Throwaway test repos have no `origin/main`; `git rev-parse` returns
        // empty after our `unwrap_or_default()` fallback.
        assert!(!is_behind_main("abc1234", ""));
        assert!(!is_behind_main("abc1234", "   \n"));
    }

    #[test]
    fn worktree_exists_matches_registered_path() {
        let entries = vec![
            WorktreeEntry {
                path: PathBuf::from("/repo"),
                branch: Some("main".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/repo/.claude/worktrees/dev-T40"),
                branch: Some("worktree-dev-T40".to_string()),
            },
        ];
        assert!(worktree_exists(
            &entries,
            Path::new("/repo/.claude/worktrees/dev-T40")
        ));
        assert!(!worktree_exists(
            &entries,
            Path::new("/repo/.claude/worktrees/dev-T41")
        ));
    }

    #[test]
    fn worktree_exists_false_on_empty_list() {
        assert!(!worktree_exists(&[], Path::new("/anywhere")));
    }

    #[test]
    fn worktree_path_for_joins_main_root() {
        let path = worktree_path_for(Path::new("/repo"), "T40");
        assert_eq!(path, PathBuf::from("/repo/.claude/worktrees/dev-T40"));
    }

    #[test]
    fn branch_name_for_prefixes_worktree() {
        assert_eq!(branch_name_for("T40"), "worktree-dev-T40");
    }
}
