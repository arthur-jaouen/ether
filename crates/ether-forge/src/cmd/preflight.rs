//! `ether-forge preflight [--task T<n>]` — verify workspace state before a
//! skill enters a worktree.
//!
//! Catches the two failures that repeatedly strand skill sessions: a dirty
//! `main` working tree (changes would be stranded outside the worktree) and a
//! stale worktree base (forces a rebase at merge time). With `--task T<n>`,
//! also refuses if an existing branch already claims the task id.
//!
//! Each check is implemented as a pure function over strings so they can be
//! unit-tested without shelling out to git.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

/// Outcome of the preflight analysis — empty `failures` means pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Report {
    pub failures: Vec<String>,
}

/// Build a report from already-collected git output. Pure, no I/O.
pub(crate) fn analyze(
    main_status: &str,
    head_branch: &str,
    main_head: &str,
    merge_base: &str,
    task: Option<&str>,
    branches: &[String],
) -> Report {
    let mut failures = Vec::new();

    let dirty = main_status.trim();
    if !dirty.is_empty() {
        let indented: String = dirty
            .lines()
            .map(|l| format!("  {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        failures.push(format!("main working tree is dirty:\n{indented}"));
    }

    if head_branch != "main" && merge_base != main_head {
        failures.push(format!(
            "branch `{head_branch}` base {} is behind main HEAD {} — rebase onto main before merging",
            short(merge_base),
            short(main_head),
        ));
    }

    if let Some(id) = task {
        let claimed = claiming_branches(branches, id);
        if !claimed.is_empty() {
            failures.push(format!(
                "existing branch already claims {id}: {}",
                claimed.join(", ")
            ));
        }
    }

    Report { failures }
}

/// Return every branch in `branches` that references task `id` as a whole
/// token (so `T35` does not match `T355`).
pub(crate) fn claiming_branches(branches: &[String], id: &str) -> Vec<String> {
    branches
        .iter()
        .filter(|b| contains_token(b, id))
        .cloned()
        .collect()
}

fn contains_token(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let nbytes = needle.as_bytes();
    if nbytes.is_empty() || bytes.len() < nbytes.len() {
        return false;
    }
    let mut i = 0;
    while i + nbytes.len() <= bytes.len() {
        if &bytes[i..i + nbytes.len()] == nbytes {
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after_idx = i + nbytes.len();
            let after_ok = after_idx == bytes.len() || !bytes[after_idx].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn short(sha: &str) -> &str {
    let end = sha.len().min(7);
    &sha[..end]
}

/// Run preflight against the real filesystem and `git` binary.
pub fn run(backlog_dir: &Path, task: Option<&str>) -> Result<()> {
    if let Some(id) = task {
        super::worktree::find_task(backlog_dir, id)?;
    }

    let main_path = main_worktree_path()?;
    let main_status = git_output(&main_path, &["status", "--porcelain"])?;

    let head_branch = git_output(Path::new("."), &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();
    let main_head = git_output(Path::new("."), &["rev-parse", "main"])?
        .trim()
        .to_string();
    let merge_base = if head_branch == "main" {
        main_head.clone()
    } else {
        git_output(Path::new("."), &["merge-base", "HEAD", "main"])?
            .trim()
            .to_string()
    };

    let branches = local_branches()?;

    let report = analyze(
        &main_status,
        &head_branch,
        &main_head,
        &merge_base,
        task,
        &branches,
    );

    if !report.failures.is_empty() {
        for f in &report.failures {
            eprintln!("preflight: {f}");
        }
        bail!("preflight failed");
    }
    println!("preflight: ok");
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

fn local_branches() -> Result<Vec<String>> {
    let raw = git_output(
        Path::new("."),
        &["for-each-ref", "--format=%(refname:short)", "refs/heads/"],
    )?;
    Ok(raw.lines().map(|s| s.to_string()).collect())
}

/// Find the primary worktree (the one `git worktree list` emits first).
fn main_worktree_path() -> Result<PathBuf> {
    let raw = git_output(Path::new("."), &["worktree", "list", "--porcelain"])?;
    for line in raw.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            return Ok(PathBuf::from(path));
        }
    }
    Err(anyhow!(
        "no worktree entries returned by `git worktree list`"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn analyze_passes_on_clean_main() {
        let report = analyze("", "main", "abc1234", "abc1234", None, &[]);
        assert!(report.failures.is_empty(), "{:?}", report);
    }

    #[test]
    fn analyze_flags_dirty_main() {
        let status = " M ROADMAP.md\n?? scratch.txt";
        let report = analyze(status, "main", "abc1234", "abc1234", None, &[]);
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].contains("dirty"));
        assert!(report.failures[0].contains("ROADMAP.md"));
        assert!(report.failures[0].contains("scratch.txt"));
    }

    #[test]
    fn analyze_flags_behind_base() {
        let report = analyze(
            "",
            "dev-T35",
            "aaaaaaa0000000",
            "bbbbbbb0000000",
            None,
            &sv(&["main", "dev-T35"]),
        );
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].contains("behind main HEAD"));
        assert!(report.failures[0].contains("aaaaaaa"));
        assert!(report.failures[0].contains("bbbbbbb"));
    }

    #[test]
    fn analyze_ignores_base_check_when_on_main() {
        // merge_base differs from main_head only because caller built it
        // differently; when head_branch is main we should not complain.
        let report = analyze("", "main", "aaaaaaa", "bbbbbbb", None, &[]);
        assert!(report.failures.is_empty(), "{:?}", report);
    }

    #[test]
    fn analyze_flags_claimed_task_branch() {
        let branches = sv(&["main", "dev-T35", "task/T9"]);
        let report = analyze("", "main", "abc", "abc", Some("T35"), &branches);
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].contains("already claims T35"));
        assert!(report.failures[0].contains("dev-T35"));
    }

    #[test]
    fn analyze_ignores_unrelated_task_branches() {
        let branches = sv(&["main", "dev-T9", "task/T355"]);
        let report = analyze("", "main", "abc", "abc", Some("T35"), &branches);
        assert!(report.failures.is_empty(), "{:?}", report);
    }

    #[test]
    fn analyze_collects_multiple_failures() {
        let branches = sv(&["worktree-dev-T35"]);
        let report = analyze(
            "?? note.md",
            "dev-T35",
            "aaaaaaa",
            "bbbbbbb",
            Some("T35"),
            &branches,
        );
        assert_eq!(report.failures.len(), 3);
    }

    #[test]
    fn claiming_branches_matches_word_boundary() {
        let branches = sv(&[
            "main",
            "dev-T35",
            "worktree-dev-T35",
            "task/T35-something",
            "dev-T350",
            "dev-T3",
            "T355-other",
        ]);
        let got = claiming_branches(&branches, "T35");
        assert_eq!(
            got,
            vec!["dev-T35", "worktree-dev-T35", "task/T35-something"]
        );
    }

    #[test]
    fn contains_token_rejects_substring_of_digits() {
        assert!(!contains_token("dev-T350", "T35"));
        assert!(!contains_token("T3590", "T35"));
        assert!(contains_token("T35", "T35"));
        assert!(contains_token("dev-T35-slug", "T35"));
    }
}
