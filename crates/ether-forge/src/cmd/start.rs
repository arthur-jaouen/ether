//! `ether-forge start` — collapse the `/dev`, `/groom`, and `/roadmap` kickoff
//! dances into one primitive. Two modes:
//!
//! Task mode (`start T<n>`) loads the task, asserts `status: ready`, reuses
//! `check` + `preflight` in-process, creates `.claude/worktrees/dev-T<n>` on
//! branch `worktree-dev-T<n>`.
//!
//! Branch mode (`start --branch <name>`) skips the backlog lookup and
//! ready-status assertion, creates `.claude/worktrees/<name>` on branch
//! `<name>`. Used by `/groom` (`groom-YYYY-MM-DD`) and `/roadmap`
//! (`roadmap-YYYY-MM-DD`).
//!
//! Both modes share the same preflight + check + `git worktree add` + fetch
//! and rebase machinery. Both also honour the in-place fallback: if the
//! primary worktree is already checked out on a non-main feature branch,
//! `start` reuses that branch instead of nesting a second worktree, and emits
//! the `mode=in-place` sentinel. This mirrors `merge`'s symmetric in-place
//! path and collapses the `/dev` skill's fresh-vs-already-on-branch dispatch
//! table into a single call.
//!
//! Every invocation ends with exactly one machine-readable sentinel line on
//! stdout — either `start: mode=created path=<abs> branch=<name>` or
//! `start: mode=in-place branch=<name>`. Skills grep for `mode=created` to
//! decide whether to follow up with `EnterWorktree`.
//!
//! Pure helpers (`is_behind_main`, `worktree_exists`, `format_sentinel`,
//! `task_mode_in_place_conflict`) are unit-tested without spawning git;
//! the top-level `run` orchestrates real git + `check` calls.
//!
//! Tests set `ETHER_FORGE_SKIP_CHECK=1` (the same seam used by `merge`) to
//! bypass the verification suite when driving `ether-forge start` against a
//! throwaway repo that has no Cargo workspace. Production callers never set
//! this — the variable is an internal test seam.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::cmd::check;
use crate::cmd::merge::{in_place_branch, parse_worktree_list, WorktreeEntry, SKIP_CHECK_ENV};
use crate::cmd::preflight::{self, claiming_branches};
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

/// Build the worktree path for a branch-mode session, rooted at the main
/// worktree. The dir name matches the branch name verbatim (e.g. `groom-2026-04-14`).
pub(crate) fn worktree_path_for_branch(main_path: &Path, branch: &str) -> PathBuf {
    main_path.join(".claude/worktrees").join(branch)
}

/// Build the branch name for task `id`.
pub(crate) fn branch_name_for(id: &str) -> String {
    format!("worktree-dev-{id}")
}

/// Outcome of a `start` invocation — drives the stable stdout sentinel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StartOutcome<'a> {
    /// A new linked worktree was created at `path` on branch `branch`.
    Created { path: &'a Path, branch: &'a str },
    /// The primary worktree's existing branch was reused in place.
    InPlace { branch: &'a str },
}

/// Format the machine-readable sentinel line. Every `start` invocation must
/// emit exactly one of these as its final stdout line — skills grep for the
/// `mode=created` marker to decide whether to follow up with `EnterWorktree`.
pub(crate) fn format_sentinel(outcome: StartOutcome<'_>) -> String {
    match outcome {
        StartOutcome::Created { path, branch } => {
            format!(
                "start: mode=created path={} branch={}",
                path.display(),
                branch
            )
        }
        StartOutcome::InPlace { branch } => {
            format!("start: mode=in-place branch={branch}")
        }
    }
}

/// Extract the first `T<digits>` token from `branch`, if any. A token is a
/// capital `T` followed by one or more ASCII digits, bounded by non-alphanumeric
/// characters (or the string boundary). Used by the task-mode in-place
/// conflict check to detect branches that already claim some OTHER task.
pub(crate) fn extract_task_id(branch: &str) -> Option<String> {
    let bytes = branch.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'T' {
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            if before_ok {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 1 {
                    let after_ok = j == bytes.len() || !bytes[j].is_ascii_alphanumeric();
                    if after_ok {
                        return Some(branch[i..j].to_string());
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Task-mode in-place fallback: given the primary worktree's `current` branch
/// and the requested task `id`, decide whether reusing the branch is safe.
/// Returns `Some(error_message)` iff the current branch clearly claims a
/// DIFFERENT task id (so silently succeeding would be wrong).
///
/// Scaffolding branches like `claude/dev-environment-setup-xyz` contain no
/// task-id token and therefore fall through to the in-place path. A branch
/// already claiming the requested id (e.g. `worktree-dev-T40` for `T40`) is
/// also fine.
pub(crate) fn task_mode_in_place_conflict(current: &str, id: &str) -> Option<String> {
    if !claiming_branches(&[current.to_string()], id).is_empty() {
        return None;
    }
    if let Some(other) = extract_task_id(current) {
        if other != id {
            return Some(format!(
                "current branch `{current}` does not claim {id} (claims {other}) — refusing to start in place"
            ));
        }
    }
    None
}

/// Run the start subcommand against the real filesystem and `git` binary.
/// Exactly one of `id` or `branch` must be `Some` — the clap layer enforces
/// this, but we still defend the invariant here.
pub fn run(
    backlog_dir: &Path,
    id: Option<&str>,
    branch: Option<&str>,
    keep_existing: bool,
) -> Result<()> {
    let (task_id, target_branch): (Option<&str>, String) = match (id, branch) {
        (Some(i), None) => {
            let task = find_task(backlog_dir, i)?;
            if task.status != Status::Ready {
                bail!("task {i} is not ready (status: {})", task.status.as_str());
            }
            (Some(i), branch_name_for(i))
        }
        (None, Some(b)) => {
            if b.is_empty() {
                bail!("--branch requires a non-empty branch name");
            }
            if b == "main" {
                bail!("refusing to start on `main`");
            }
            (None, b.to_string())
        }
        _ => bail!("start requires exactly one of a task id or `--branch <name>`"),
    };

    let cwd = std::env::current_dir().context("reading current directory")?;
    let list_raw = git_output(&cwd, &["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&list_raw);
    let main_path = entries
        .first()
        .map(|e| e.path.clone())
        .ok_or_else(|| anyhow!("no git worktrees listed"))?;

    // In-place fallback: primary worktree is on a non-main feature branch.
    // Skip the claim check in preflight (the branch is already there) and
    // the claim-check would trip on it anyway.
    if let Some(current) = in_place_branch(&entries) {
        if let Some(id) = task_id {
            if let Some(msg) = task_mode_in_place_conflict(&current, id) {
                bail!(msg);
            }
        }
        // Branch mode is permissive: any non-main branch qualifies, matching
        // the Claude Code on the Web scaffolding use case where `/groom`
        // wants to work in place regardless of the inherited branch name.
        preflight::run(backlog_dir, None).context("preflight failed — refusing to start")?;
        if std::env::var_os(SKIP_CHECK_ENV).is_none() {
            check::run().context("ether-forge check failed — start aborted")?;
        }
        println!("start: already on {current}, skipping worktree creation");
        println!(
            "{}",
            format_sentinel(StartOutcome::InPlace { branch: &current })
        );
        return Ok(());
    }

    // Fresh path: create a new linked worktree.
    let wt_path = match task_id {
        Some(i) => worktree_path_for(&main_path, i),
        None => worktree_path_for_branch(&main_path, &target_branch),
    };
    let wt_already_registered = worktree_exists(&entries, &wt_path);
    let wt_dir_exists = wt_path.exists();

    if wt_already_registered && !keep_existing {
        bail!(
            "worktree {} already exists (pass --keep-existing to reuse)",
            wt_path.display()
        );
    }
    if wt_dir_exists && !wt_already_registered && !keep_existing {
        bail!(
            "path {} already exists on disk but is not a registered worktree (pass --keep-existing to reuse)",
            wt_path.display()
        );
    }

    // When reusing an existing worktree, skip the preflight claim check —
    // the branch we're about to reuse will trip it, and the check is there
    // to refuse NEW claims, not existing ones.
    let preflight_task = if wt_already_registered && keep_existing {
        None
    } else {
        task_id
    };
    preflight::run(backlog_dir, preflight_task).context("preflight failed — refusing to start")?;
    if std::env::var_os(SKIP_CHECK_ENV).is_none() {
        check::run().context("ether-forge check failed — start aborted")?;
    }

    if let Some(parent) = wt_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating worktree parent dir {}", parent.display()))?;
    }

    let wt_str = wt_path
        .to_str()
        .ok_or_else(|| anyhow!("worktree path is not UTF-8"))?;

    if !wt_already_registered {
        let add = Command::new("git")
            .current_dir(&main_path)
            .args(["worktree", "add", "-b", &target_branch, wt_str, "main"])
            .status()
            .context("spawning git worktree add")?;
        if !add.success() {
            bail!("`git worktree add` failed for {}", wt_path.display());
        }
    } else {
        println!(
            "start: reusing existing worktree at {} (--keep-existing)",
            wt_path.display()
        );
    }

    // Fetch and rebase from inside the worktree. We skip the fetch when
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
        println!("start: rebasing {target_branch} onto origin/main");
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
    println!("start: on branch {target_branch}");
    if let Some(i) = task_id {
        println!("start: next → cd {} && implement {i}", wt_path.display());
    } else {
        println!("start: next → cd {}", wt_path.display());
    }
    println!(
        "{}",
        format_sentinel(StartOutcome::Created {
            path: &wt_path,
            branch: &target_branch,
        })
    );
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
    fn worktree_path_for_branch_joins_main_root() {
        let path = worktree_path_for_branch(Path::new("/repo"), "groom-2026-04-14");
        assert_eq!(
            path,
            PathBuf::from("/repo/.claude/worktrees/groom-2026-04-14")
        );
    }

    #[test]
    fn branch_name_for_prefixes_worktree() {
        assert_eq!(branch_name_for("T40"), "worktree-dev-T40");
    }

    #[test]
    fn format_sentinel_created_path_and_branch() {
        let path = PathBuf::from("/repo/.claude/worktrees/dev-T40");
        let s = format_sentinel(StartOutcome::Created {
            path: &path,
            branch: "worktree-dev-T40",
        });
        assert_eq!(
            s,
            "start: mode=created path=/repo/.claude/worktrees/dev-T40 branch=worktree-dev-T40"
        );
    }

    #[test]
    fn format_sentinel_in_place_branch_only() {
        let s = format_sentinel(StartOutcome::InPlace {
            branch: "claude/dev-environment-setup-xyz",
        });
        assert_eq!(
            s,
            "start: mode=in-place branch=claude/dev-environment-setup-xyz"
        );
    }

    #[test]
    fn extract_task_id_finds_simple_token() {
        assert_eq!(extract_task_id("dev-T17"), Some("T17".to_string()));
        assert_eq!(extract_task_id("worktree-dev-T40"), Some("T40".to_string()));
        assert_eq!(extract_task_id("claude/dev-T7-xyz"), Some("T7".to_string()));
    }

    #[test]
    fn extract_task_id_rejects_non_boundary() {
        assert_eq!(extract_task_id("dev-xT17"), None);
        assert_eq!(extract_task_id("dev-T17a"), None);
    }

    #[test]
    fn extract_task_id_none_for_scaffolding_branches() {
        assert_eq!(extract_task_id("claude/dev-environment-setup-xyz"), None);
        assert_eq!(extract_task_id("main"), None);
        assert_eq!(extract_task_id("groom-2026-04-14"), None);
    }

    #[test]
    fn task_mode_conflict_flags_other_task_branch() {
        let err = task_mode_in_place_conflict("dev-T17", "T40").unwrap();
        assert!(err.contains("does not claim T40"));
        assert!(err.contains("claims T17"));
    }

    #[test]
    fn task_mode_conflict_allows_matching_branch() {
        assert!(task_mode_in_place_conflict("dev-T40", "T40").is_none());
        assert!(task_mode_in_place_conflict("worktree-dev-T40", "T40").is_none());
    }

    #[test]
    fn task_mode_conflict_allows_scaffolding_branch() {
        assert!(task_mode_in_place_conflict("claude/dev-environment-setup-xyz", "T40").is_none());
        assert!(task_mode_in_place_conflict("feature/new-thing", "T40").is_none());
    }
}
