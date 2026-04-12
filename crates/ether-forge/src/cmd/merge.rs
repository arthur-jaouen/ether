//! `ether-forge merge T<n>` — collapse the skill wrap-up dance into one
//! primitive. Verify the worktree is clean, rebase onto `main` if it
//! advanced, re-run `check`, apply the reviewer-blocker gate, fast-forward
//! `main`, then remove the worktree directory and delete the branch (unless
//! `--keep` is set). The exit-side mirror of `preflight`.
//!
//! Pure helpers (`is_clean`, `is_behind`, `parse_worktree_list`,
//! `resolve_worktree`) are unit-tested without spawning git; the top-level
//! `run` orchestrates real git + `check` calls and handles edge cases like a
//! pre-removed worktree directory.
//!
//! Tests set `ETHER_FORGE_SKIP_CHECK=1` to bypass the verification suite
//! when driving `ether-forge merge` against a throwaway repo that has no
//! Cargo workspace. Production callers never set this — the variable is an
//! internal test seam.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::cmd::check;
use crate::cmd::commit::{evaluate_gate, load_artifact, review_artifact_path};
use crate::cmd::preflight::claiming_branches;
use crate::task::find_task;

/// Name of the env var that short-circuits `check::run()` during integration
/// tests that run against a tempdir with no Cargo workspace.
pub(crate) const SKIP_CHECK_ENV: &str = "ETHER_FORGE_SKIP_CHECK";

/// Worktree is clean iff `git status --porcelain` produced no entries.
pub(crate) fn is_clean(status: &str) -> bool {
    status.trim().is_empty()
}

/// Branch is behind main iff its merge-base with main differs from main HEAD.
pub(crate) fn is_behind(main_head: &str, merge_base: &str) -> bool {
    main_head.trim() != merge_base.trim()
}

/// A single entry parsed from `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeEntry {
    pub path: PathBuf,
    pub branch: Option<String>,
}

/// Parse `git worktree list --porcelain` into entries. Detached worktrees
/// appear with `branch: None`.
pub(crate) fn parse_worktree_list(raw: &str) -> Vec<WorktreeEntry> {
    let mut out = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    for line in raw.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            if let Some(pb) = path.take() {
                out.push(WorktreeEntry {
                    path: pb,
                    branch: branch.take(),
                });
            }
            path = Some(PathBuf::from(p));
        } else if let Some(b) = line.strip_prefix("branch ") {
            let short = b.strip_prefix("refs/heads/").unwrap_or(b);
            branch = Some(short.to_string());
        }
    }
    if let Some(pb) = path.take() {
        out.push(WorktreeEntry {
            path: pb,
            branch: branch.take(),
        });
    }
    out
}

/// How to match a worktree to a merge target.
pub(crate) enum Match<'a> {
    /// Match the unique worktree whose branch claims `id` as a word token.
    TaskId(&'a str),
    /// Match the worktree whose branch equals `name` exactly.
    BranchName(&'a str),
}

/// Pick the worktree to merge. With `explicit` set, match by path; otherwise
/// use the [`Match`] strategy.
pub(crate) fn resolve_worktree(
    entries: &[WorktreeEntry],
    how: Match<'_>,
    explicit: Option<&Path>,
) -> Result<WorktreeEntry> {
    if let Some(path) = explicit {
        for e in entries {
            if e.path == path {
                return Ok(e.clone());
            }
        }
        bail!(
            "--worktree {} is not registered with git worktree list",
            path.display()
        );
    }
    let candidates: Vec<&WorktreeEntry> = entries
        .iter()
        .filter(|e| e.branch.as_deref() != Some("main"))
        .filter(|e| match (&e.branch, &how) {
            (Some(b), Match::TaskId(id)) => {
                !claiming_branches(std::slice::from_ref(b), id).is_empty()
            }
            (Some(b), Match::BranchName(name)) => b == name,
            (None, _) => false,
        })
        .collect();
    let target_label = match how {
        Match::TaskId(id) => id.to_string(),
        Match::BranchName(n) => format!("branch {n}"),
    };
    match candidates.len() {
        0 => Err(anyhow!("no worktree found claiming {target_label}")),
        1 => Ok(candidates[0].clone()),
        _ => Err(anyhow!(
            "multiple worktrees claim {target_label}: {}",
            candidates
                .iter()
                .map(|e| e.path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Heuristic: does `target` look like a backlog task id (`T` followed by
/// digits)? Controls whether merge runs the review-gate check.
pub(crate) fn looks_like_task_id(target: &str) -> bool {
    if let Some(rest) = target.strip_prefix('T') {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

/// Run the merge subcommand against the real filesystem and `git` binary.
/// `target` may be either a backlog task id (`T38`) or an explicit branch
/// name (`groom-2026-04-13`). The review-gate check only fires for task-id
/// targets.
pub fn run(
    backlog_dir: &Path,
    target: &str,
    keep: bool,
    force_review: bool,
    worktree: Option<&Path>,
) -> Result<()> {
    let (task_id, how): (Option<String>, Match<'_>) = if looks_like_task_id(target) {
        let task = find_task(backlog_dir, target)?;
        let id = task.id.clone();
        (Some(id), Match::TaskId(target))
    } else {
        (None, Match::BranchName(target))
    };

    let cwd = std::env::current_dir().context("reading current directory")?;
    let list_raw = git_output(&cwd, &["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&list_raw);
    let main_path = entries
        .first()
        .map(|e| e.path.clone())
        .ok_or_else(|| anyhow!("no git worktrees listed"))?;

    let target_entry = resolve_worktree(&entries, how, worktree)?;
    let target = target_entry;
    let branch = target
        .branch
        .clone()
        .ok_or_else(|| anyhow!("target worktree has no branch (detached HEAD)"))?;
    if branch == "main" {
        bail!("refusing to merge main into itself");
    }
    let wt_exists = target.path.exists();

    if wt_exists {
        let status = git_output(&target.path, &["status", "--porcelain"])?;
        if !is_clean(&status) {
            bail!(
                "worktree {} is dirty — commit or stash before merging",
                target.path.display()
            );
        }

        let main_head = git_output(&main_path, &["rev-parse", "main"])?
            .trim()
            .to_string();
        let merge_base = git_output(&target.path, &["merge-base", "HEAD", "main"])?
            .trim()
            .to_string();
        if is_behind(&main_head, &merge_base) {
            println!("merge: rebasing {branch} onto main");
            let status = Command::new("git")
                .current_dir(&target.path)
                .args(["rebase", "main"])
                .status()
                .context("spawning git rebase")?;
            if !status.success() {
                bail!(
                    "`git rebase main` failed in {} — resolve conflicts and re-run",
                    target.path.display()
                );
            }
        }

        if std::env::var_os(SKIP_CHECK_ENV).is_none() {
            let original = std::env::current_dir().context("reading current directory")?;
            std::env::set_current_dir(&target.path)
                .with_context(|| format!("chdir {}", target.path.display()))?;
            let result = check::run().context("ether-forge check failed — merge aborted");
            let _ = std::env::set_current_dir(&original);
            result?;
        }
    } else {
        println!(
            "merge: worktree directory missing at {} — skipping clean/rebase/check",
            target.path.display()
        );
    }

    if let Some(id) = task_id.as_deref() {
        let artifact_path = review_artifact_path(&main_path.join("target"), id);
        let artifact = load_artifact(&artifact_path)?;
        evaluate_gate(artifact.as_ref(), id, force_review)?;
    }

    let status = Command::new("git")
        .current_dir(&main_path)
        .args(["merge", "--ff-only", &branch])
        .status()
        .context("spawning git merge")?;
    if !status.success() {
        bail!("`git merge --ff-only {branch}` failed — main may have diverged");
    }

    if keep {
        println!("merge: ff-merged {branch} into main (kept worktree + branch)");
        return Ok(());
    }

    if wt_exists {
        let remove = Command::new("git")
            .current_dir(&main_path)
            .args([
                "worktree",
                "remove",
                target
                    .path
                    .to_str()
                    .ok_or_else(|| anyhow!("worktree path is not UTF-8"))?,
            ])
            .status()
            .context("spawning git worktree remove")?;
        if !remove.success() {
            let _ = std::fs::remove_dir_all(&target.path);
        }
    }
    let _ = Command::new("git")
        .current_dir(&main_path)
        .args(["worktree", "prune"])
        .status();

    let del = Command::new("git")
        .current_dir(&main_path)
        .args(["branch", "-d", &branch])
        .status()
        .context("spawning git branch -d")?;
    if !del.success() {
        let force = Command::new("git")
            .current_dir(&main_path)
            .args(["branch", "-D", &branch])
            .status()
            .context("spawning git branch -D")?;
        if !force.success() {
            bail!("could not delete branch {branch}");
        }
    }

    println!("merge: ff-merged {branch} into main and cleaned up worktree");
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
    fn is_clean_true_on_empty() {
        assert!(is_clean(""));
        assert!(is_clean("   \n"));
    }

    #[test]
    fn is_clean_false_on_any_entry() {
        assert!(!is_clean(" M file.rs\n"));
        assert!(!is_clean("?? scratch.txt"));
    }

    #[test]
    fn is_behind_detects_divergence() {
        assert!(is_behind("aaa111", "bbb222"));
        assert!(!is_behind("abc1234", "abc1234"));
        assert!(!is_behind("abc1234\n", "abc1234"));
    }

    #[test]
    fn parse_worktree_list_handles_primary_and_branch() {
        let raw = "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /repo/.claude/worktrees/dev-T38\nHEAD def\nbranch refs/heads/dev-T38\n";
        let entries = parse_worktree_list(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, PathBuf::from("/repo"));
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert_eq!(
            entries[1].path,
            PathBuf::from("/repo/.claude/worktrees/dev-T38")
        );
        assert_eq!(entries[1].branch.as_deref(), Some("dev-T38"));
    }

    #[test]
    fn parse_worktree_list_handles_detached_head() {
        let raw = "worktree /repo\nHEAD abc\ndetached\n";
        let entries = parse_worktree_list(raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].branch, None);
    }

    #[test]
    fn resolve_worktree_prefers_explicit_path() {
        let entries = vec![
            WorktreeEntry {
                path: PathBuf::from("/repo"),
                branch: Some("main".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt/a"),
                branch: Some("dev-T38".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt/b"),
                branch: Some("feature-T38".to_string()),
            },
        ];
        let got =
            resolve_worktree(&entries, Match::TaskId("T38"), Some(Path::new("/wt/b"))).unwrap();
        assert_eq!(got.path, PathBuf::from("/wt/b"));
    }

    #[test]
    fn resolve_worktree_errors_when_explicit_missing() {
        let entries = vec![WorktreeEntry {
            path: PathBuf::from("/repo"),
            branch: Some("main".to_string()),
        }];
        let err =
            resolve_worktree(&entries, Match::TaskId("T38"), Some(Path::new("/nope"))).unwrap_err();
        assert!(format!("{err:#}").contains("not registered"));
    }

    #[test]
    fn resolve_worktree_picks_unique_claimant() {
        let entries = vec![
            WorktreeEntry {
                path: PathBuf::from("/repo"),
                branch: Some("main".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt"),
                branch: Some("worktree-dev-T38".to_string()),
            },
        ];
        let got = resolve_worktree(&entries, Match::TaskId("T38"), None).unwrap();
        assert_eq!(got.path, PathBuf::from("/wt"));
    }

    #[test]
    fn resolve_worktree_ignores_digit_substring_branches() {
        let entries = vec![
            WorktreeEntry {
                path: PathBuf::from("/repo"),
                branch: Some("main".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt-other"),
                branch: Some("dev-T380".to_string()),
            },
        ];
        let err = resolve_worktree(&entries, Match::TaskId("T38"), None).unwrap_err();
        assert!(format!("{err:#}").contains("no worktree"));
    }

    #[test]
    fn resolve_worktree_errors_on_ambiguous_claim() {
        let entries = vec![
            WorktreeEntry {
                path: PathBuf::from("/repo"),
                branch: Some("main".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt-a"),
                branch: Some("dev-T38".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt-b"),
                branch: Some("fix-T38-bug".to_string()),
            },
        ];
        let err = resolve_worktree(&entries, Match::TaskId("T38"), None).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("multiple"));
        assert!(msg.contains("/wt-a"));
        assert!(msg.contains("/wt-b"));
    }

    #[test]
    fn resolve_worktree_rejects_when_only_main_matches() {
        // Defensive: we explicitly skip the `main` branch so an id that
        // happens to appear in a local tag or the main branch name never
        // accidentally targets the primary worktree.
        let entries = vec![WorktreeEntry {
            path: PathBuf::from("/repo"),
            branch: Some("main".to_string()),
        }];
        assert!(resolve_worktree(&entries, Match::TaskId("T38"), None).is_err());
    }

    #[test]
    fn resolve_worktree_matches_exact_branch_name() {
        let entries = vec![
            WorktreeEntry {
                path: PathBuf::from("/repo"),
                branch: Some("main".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt-groom"),
                branch: Some("groom-2026-04-13".to_string()),
            },
            WorktreeEntry {
                path: PathBuf::from("/wt-other"),
                branch: Some("groom-2026-04-14".to_string()),
            },
        ];
        let got = resolve_worktree(&entries, Match::BranchName("groom-2026-04-13"), None).unwrap();
        assert_eq!(got.path, PathBuf::from("/wt-groom"));
    }

    #[test]
    fn looks_like_task_id_accepts_tn_form() {
        assert!(looks_like_task_id("T1"));
        assert!(looks_like_task_id("T38"));
        assert!(looks_like_task_id("T1000"));
    }

    #[test]
    fn looks_like_task_id_rejects_other_shapes() {
        assert!(!looks_like_task_id("T"));
        assert!(!looks_like_task_id("t38"));
        assert!(!looks_like_task_id("T38-x"));
        assert!(!looks_like_task_id("groom-2026-04-13"));
        assert!(!looks_like_task_id("dev-T38"));
    }
}
