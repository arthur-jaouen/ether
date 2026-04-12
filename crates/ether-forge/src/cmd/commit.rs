//! `ether-forge commit T<n>` — run `check`, then `git commit` with a
//! task-derived message.
//!
//! The commit message is `T<n>: <title>` pulled from the task's frontmatter.
//! Before invoking git, the commit gate reads
//! `target/.ether-forge/review-T<n>.json` (if it exists) and refuses the
//! commit when the artifact lists any blockers. `--force-review` bypasses the
//! gate and appends a `Reviewed-by-force: true` trailer so the override is
//! recorded in the commit message.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::cmd::check;
use crate::task::find_task;

const FORCE_TRAILER: &str = "Reviewed-by-force: true";

/// Shape of `target/.ether-forge/review-T<n>.json`. Only `blockers` is
/// load-bearing for the gate; other fields the reviewer emits are ignored.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct ReviewArtifact {
    #[serde(default)]
    pub blockers: Vec<String>,
}

/// Outcome of the review gate — decides whether the commit proceeds and
/// whether a force trailer should be appended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GateOutcome {
    /// No artifact, or artifact with an empty `blockers` list.
    Clean,
    /// Artifact had blockers but `--force-review` was set.
    Forced,
}

/// Conventional artifact path for a task id, rooted at `target/`.
pub(crate) fn review_artifact_path(target_root: &Path, id: &str) -> PathBuf {
    target_root
        .join(".ether-forge")
        .join(format!("review-{id}.json"))
}

/// Read the review artifact. Returns `Ok(None)` when the file is absent so
/// the gate stays silent for tasks without a review on disk.
pub(crate) fn load_artifact(path: &Path) -> Result<Option<ReviewArtifact>> {
    match std::fs::read_to_string(path) {
        Ok(body) => {
            let parsed: ReviewArtifact = serde_json::from_str(&body)
                .with_context(|| format!("parsing review artifact at {}", path.display()))?;
            Ok(Some(parsed))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(anyhow::Error::new(err)
            .context(format!("reading review artifact at {}", path.display()))),
    }
}

/// Core gate logic: pure over an artifact + force flag so it is directly
/// unit-testable without spawning git.
pub(crate) fn evaluate_gate(
    artifact: Option<&ReviewArtifact>,
    id: &str,
    force: bool,
) -> Result<GateOutcome> {
    let blockers = artifact.map(|a| a.blockers.as_slice()).unwrap_or_default();
    if blockers.is_empty() {
        return Ok(GateOutcome::Clean);
    }
    if force {
        return Ok(GateOutcome::Forced);
    }
    let mut report = format!(
        "reviewer flagged {} blocker(s) for {id} — commit refused\n",
        blockers.len()
    );
    for b in blockers {
        report.push_str("  - ");
        report.push_str(b);
        report.push('\n');
    }
    report.push_str("re-run after addressing findings, or pass `--force-review` to override");
    Err(anyhow!(report))
}

/// Assemble the `git commit` argv with a message and extra passthrough args.
/// When `force_trailer` is set, a `-m` is appended carrying the force trailer
/// so it lands as its own paragraph in the commit message.
pub(crate) fn commit_argv<'a>(
    message: &'a str,
    extra: &'a [String],
    force_trailer: bool,
) -> Vec<&'a str> {
    let mut argv: Vec<&str> = vec!["git", "commit", "-m", message];
    for a in extra {
        argv.push(a.as_str());
    }
    if force_trailer {
        argv.push("-m");
        argv.push(FORCE_TRAILER);
    }
    argv
}

/// Run the commit subcommand against the real binaries.
pub fn run(backlog_dir: &Path, id: &str, extra: &[String], force_review: bool) -> Result<()> {
    check::run().context("ether-forge check failed — commit aborted")?;
    let task = find_task(backlog_dir, id)?;
    let artifact_path = review_artifact_path(Path::new("target"), &task.id);
    let artifact = load_artifact(&artifact_path)?;
    let outcome = evaluate_gate(artifact.as_ref(), &task.id, force_review)?;
    let message = format!("{}: {}", task.id, task.title);
    let argv = commit_argv(&message, extra, outcome == GateOutcome::Forced);
    let status = spawn_real(&argv)?;
    if !status.success() {
        bail!("`{}` failed with {}", argv.join(" "), status);
    }
    Ok(())
}

fn spawn_real(argv: &[&str]) -> Result<ExitStatus> {
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

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn commit_argv_basic_message() {
        let extra: Vec<String> = Vec::new();
        let argv = commit_argv("T9: title here", &extra, false);
        assert_eq!(argv, vec!["git", "commit", "-m", "T9: title here"]);
    }

    #[test]
    fn commit_argv_forwards_extra_args() {
        let extra = vec!["-a".to_string(), "-m".to_string(), "more".to_string()];
        let argv = commit_argv("T9: x", &extra, false);
        assert_eq!(
            argv,
            vec!["git", "commit", "-m", "T9: x", "-a", "-m", "more"]
        );
    }

    #[test]
    fn commit_argv_appends_force_trailer_as_separate_paragraph() {
        let extra = vec!["-a".to_string()];
        let argv = commit_argv("T9: x", &extra, true);
        assert_eq!(
            argv,
            vec!["git", "commit", "-m", "T9: x", "-a", "-m", FORCE_TRAILER]
        );
    }

    #[test]
    fn gate_passes_when_no_artifact() {
        let outcome = evaluate_gate(None, "T29", false).expect("clean gate");
        assert_eq!(outcome, GateOutcome::Clean);
    }

    #[test]
    fn gate_passes_when_blockers_empty() {
        let artifact = ReviewArtifact { blockers: vec![] };
        let outcome = evaluate_gate(Some(&artifact), "T29", false).expect("clean gate");
        assert_eq!(outcome, GateOutcome::Clean);
    }

    #[test]
    fn gate_refuses_when_blockers_present() {
        let artifact = ReviewArtifact {
            blockers: vec!["missing SAFETY comment on unsafe block".to_string()],
        };
        let err =
            evaluate_gate(Some(&artifact), "T29", false).expect_err("blocker must refuse commit");
        let msg = format!("{err}");
        assert!(msg.contains("T29"));
        assert!(msg.contains("missing SAFETY comment"));
        assert!(msg.contains("--force-review"));
    }

    #[test]
    fn gate_force_review_returns_forced() {
        let artifact = ReviewArtifact {
            blockers: vec!["nondeterministic HashMap iteration".to_string()],
        };
        let outcome = evaluate_gate(Some(&artifact), "T29", true).expect("force must bypass gate");
        assert_eq!(outcome, GateOutcome::Forced);
    }

    #[test]
    fn load_artifact_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let path = review_artifact_path(tmp.path(), "T29");
        assert!(load_artifact(&path).unwrap().is_none());
    }

    #[test]
    fn load_artifact_reads_blockers_field() {
        let tmp = TempDir::new().unwrap();
        let path = review_artifact_path(tmp.path(), "T29");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"blockers":["a","b"],"nits":["ignored"]}"#).unwrap();
        let artifact = load_artifact(&path).unwrap().expect("file present");
        assert_eq!(artifact.blockers, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn load_artifact_reports_parse_errors() {
        let tmp = TempDir::new().unwrap();
        let path = review_artifact_path(tmp.path(), "T29");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        assert!(load_artifact(&path).is_err());
    }
}
