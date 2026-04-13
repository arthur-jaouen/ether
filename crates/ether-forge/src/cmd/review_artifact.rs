//! `ether-forge review-artifact` — write the canonical reviewer artifact.
//!
//! The reviewer subagent used to hand-roll `mkdir -p target/.ether-forge`
//! followed by a `Write` of a JSON blob it composed itself. That contract
//! drifted easily because the schema lived in prose. This subcommand owns
//! the schema mechanically:
//!
//! * `--blocker file:line:message` and `--nit file:line:message` (repeated)
//!   build entries from CLI args. Malformed inputs are rejected with a clear
//!   error before anything is written.
//! * `--from-stdin` reads a pre-built JSON payload, validates every entry
//!   has `file`/`line`/`message`, and re-emits a canonicalized form.
//!
//! Either way, the result lands at `target/.ether-forge/review-T<n>.json`,
//! creating parent directories as needed. `commit::run` reads the same file
//! to enforce the blocker gate, so this module also defines the schema
//! types that downstream consumers import.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

/// One reviewer finding pinned to a source location.
///
/// `line` is `0` when the finding is not tied to a specific line (e.g. a
/// file-level concern). `file` and `message` must be non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewEntry {
    pub file: String,
    pub line: u32,
    pub message: String,
}

/// Canonical reviewer artifact shape consumed by the commit gate.
///
/// `blockers` is load-bearing: any entry refuses the commit unless
/// `--force-review` is passed. `nits` are advisory and never gate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewArtifact {
    #[serde(default)]
    pub blockers: Vec<ReviewEntry>,
    #[serde(default)]
    pub nits: Vec<ReviewEntry>,
}

impl ReviewArtifact {
    /// Reject entries with empty `file` or `message`. Called on every write
    /// path so a malformed payload never reaches disk.
    pub fn validate(&self) -> Result<()> {
        for (kind, entry) in self
            .blockers
            .iter()
            .map(|e| ("blocker", e))
            .chain(self.nits.iter().map(|e| ("nit", e)))
        {
            if entry.file.trim().is_empty() {
                bail!("{kind} entry has empty `file`");
            }
            if entry.message.trim().is_empty() {
                bail!("{kind} entry has empty `message`");
            }
        }
        Ok(())
    }
}

/// Conventional artifact path for a task id, rooted at `target/`.
pub fn artifact_path(target_root: &Path, id: &str) -> PathBuf {
    target_root
        .join(".ether-forge")
        .join(format!("review-{id}.json"))
}

/// Parse a `file:line:message` CLI triple. The first two `:` separators are
/// the field boundaries; any further colons remain inside `message` so paths
/// or rustdoc-style links survive intact.
pub fn parse_entry(spec: &str) -> Result<ReviewEntry> {
    let (file, rest) = spec
        .split_once(':')
        .ok_or_else(|| anyhow!("`{spec}`: expected `file:line:message`, missing `:` after file"))?;
    let (line_str, message) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("`{spec}`: expected `file:line:message`, missing `:` after line"))?;
    if file.trim().is_empty() {
        bail!("`{spec}`: empty `file` field");
    }
    let line: u32 = line_str.parse().with_context(|| {
        format!("`{spec}`: `line` must be a non-negative integer, got `{line_str}`")
    })?;
    if message.trim().is_empty() {
        bail!("`{spec}`: empty `message` field");
    }
    Ok(ReviewEntry {
        file: file.to_string(),
        line,
        message: message.to_string(),
    })
}

/// Build a `ReviewArtifact` from repeated CLI flags.
pub fn build_from_cli(blockers: &[String], nits: &[String]) -> Result<ReviewArtifact> {
    let blockers = blockers
        .iter()
        .map(|s| parse_entry(s))
        .collect::<Result<Vec<_>>>()?;
    let nits = nits
        .iter()
        .map(|s| parse_entry(s))
        .collect::<Result<Vec<_>>>()?;
    Ok(ReviewArtifact { blockers, nits })
}

/// Parse a JSON payload into a `ReviewArtifact` and validate every entry.
pub fn parse_from_json(body: &str) -> Result<ReviewArtifact> {
    let parsed: ReviewArtifact =
        serde_json::from_str(body).context("parsing review artifact JSON")?;
    parsed.validate()?;
    Ok(parsed)
}

/// Write a validated artifact to `target/.ether-forge/review-<id>.json`,
/// creating parent directories as needed. The on-disk form is pretty-printed
/// JSON with a trailing newline so diffs stay readable.
pub fn write_artifact(target_root: &Path, id: &str, artifact: &ReviewArtifact) -> Result<PathBuf> {
    artifact.validate()?;
    let path = artifact_path(target_root, id);
    let parent = path
        .parent()
        .expect("artifact_path always nests under .ether-forge");
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let mut body = serde_json::to_string_pretty(artifact).context("serializing review artifact")?;
    body.push('\n');
    std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Run the subcommand against the real filesystem and stdin.
pub fn run(
    target_root: &Path,
    id: &str,
    blockers: &[String],
    nits: &[String],
    from_stdin: bool,
) -> Result<()> {
    if from_stdin && (!blockers.is_empty() || !nits.is_empty()) {
        bail!("`--from-stdin` is mutually exclusive with `--blocker` / `--nit`");
    }
    let artifact = if from_stdin {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading review artifact from stdin")?;
        parse_from_json(&buf)?
    } else {
        build_from_cli(blockers, nits)?
    };
    let path = write_artifact(target_root, id, &artifact)?;
    println!("{}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn entry(file: &str, line: u32, message: &str) -> ReviewEntry {
        ReviewEntry {
            file: file.to_string(),
            line,
            message: message.to_string(),
        }
    }

    #[test]
    fn parse_entry_splits_three_fields() {
        let e = parse_entry("src/foo.rs:42:missing SAFETY comment").unwrap();
        assert_eq!(e, entry("src/foo.rs", 42, "missing SAFETY comment"));
    }

    #[test]
    fn parse_entry_keeps_colons_inside_message() {
        let e = parse_entry("src/foo.rs:7:see crate::bar::baz for context").unwrap();
        assert_eq!(e, entry("src/foo.rs", 7, "see crate::bar::baz for context"));
    }

    #[test]
    fn parse_entry_allows_line_zero_for_file_level_findings() {
        let e = parse_entry("crates/x/Cargo.toml:0:missing version field").unwrap();
        assert_eq!(e.line, 0);
    }

    #[test]
    fn parse_entry_rejects_missing_line_separator() {
        let err = parse_entry("src/foo.rs").unwrap_err();
        assert!(err.to_string().contains("missing `:` after file"));
    }

    #[test]
    fn parse_entry_rejects_missing_message_separator() {
        let err = parse_entry("src/foo.rs:42").unwrap_err();
        assert!(err.to_string().contains("missing `:` after line"));
    }

    #[test]
    fn parse_entry_rejects_non_numeric_line() {
        let err = parse_entry("src/foo.rs:abc:msg").unwrap_err();
        assert!(err.to_string().contains("must be a non-negative integer"));
    }

    #[test]
    fn parse_entry_rejects_empty_file() {
        let err = parse_entry(":42:msg").unwrap_err();
        assert!(err.to_string().contains("empty `file`"));
    }

    #[test]
    fn parse_entry_rejects_empty_message() {
        let err = parse_entry("src/foo.rs:42:").unwrap_err();
        assert!(err.to_string().contains("empty `message`"));
    }

    #[test]
    fn validate_rejects_whitespace_only_fields() {
        let bad = ReviewArtifact {
            blockers: vec![entry("   ", 1, "msg")],
            nits: vec![],
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn write_artifact_creates_parent_directory() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("target");
        let artifact = ReviewArtifact::default();
        let path = write_artifact(&target, "T43", &artifact).unwrap();
        assert!(path.exists());
        assert_eq!(path, artifact_path(&target, "T43"));
        let body = std::fs::read_to_string(&path).unwrap();
        let round: ReviewArtifact = serde_json::from_str(&body).unwrap();
        assert_eq!(round, artifact);
        assert!(body.ends_with('\n'));
    }

    #[test]
    fn write_artifact_round_trips_blockers_and_nits() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("target");
        let artifact = ReviewArtifact {
            blockers: vec![entry("a.rs", 1, "boom")],
            nits: vec![entry("b.rs", 0, "tiny")],
        };
        let path = write_artifact(&target, "T43", &artifact).unwrap();
        let round: ReviewArtifact =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(round, artifact);
    }

    #[test]
    fn write_artifact_validates_before_writing() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("target");
        let bad = ReviewArtifact {
            blockers: vec![entry("a.rs", 1, "")],
            nits: vec![],
        };
        let err = write_artifact(&target, "T43", &bad).unwrap_err();
        assert!(err.to_string().contains("empty `message`"));
        // No file should have been created.
        assert!(!artifact_path(&target, "T43").exists());
    }

    #[test]
    fn build_from_cli_collects_entries() {
        let artifact = build_from_cli(
            &["a.rs:1:one".into(), "b.rs:2:two".into()],
            &["c.rs:0:three".into()],
        )
        .unwrap();
        assert_eq!(artifact.blockers.len(), 2);
        assert_eq!(artifact.nits.len(), 1);
        assert_eq!(artifact.blockers[1], entry("b.rs", 2, "two"));
    }

    #[test]
    fn build_from_cli_propagates_first_parse_error() {
        let err = build_from_cli(&["a.rs:1:ok".into(), "broken".into()], &[]).unwrap_err();
        assert!(err.to_string().contains("broken"));
    }

    #[test]
    fn parse_from_json_accepts_canonical_payload() {
        let body = r#"{
            "blockers": [{"file": "a.rs", "line": 1, "message": "boom"}],
            "nits": []
        }"#;
        let artifact = parse_from_json(body).unwrap();
        assert_eq!(artifact.blockers[0], entry("a.rs", 1, "boom"));
        assert!(artifact.nits.is_empty());
    }

    #[test]
    fn parse_from_json_defaults_missing_arrays_to_empty() {
        let artifact = parse_from_json("{}").unwrap();
        assert!(artifact.blockers.is_empty());
        assert!(artifact.nits.is_empty());
    }

    #[test]
    fn parse_from_json_validates_entries() {
        let body = r#"{"blockers": [{"file": "", "line": 1, "message": "x"}]}"#;
        let err = parse_from_json(body).unwrap_err();
        assert!(err.to_string().contains("empty `file`"));
    }

    #[test]
    fn parse_from_json_reports_parse_errors() {
        let err = parse_from_json("not json").unwrap_err();
        assert!(err.to_string().contains("parsing review artifact JSON"));
    }

    /// Contract lint: every `--blocker "..."` / `--nit "..."` invocation
    /// shown in `.claude/agents/reviewer.md` must round-trip through
    /// `parse_entry` unchanged. The reviewer agent copies these strings as
    /// templates; if the CLI parser tightens and the doc isn't updated, a
    /// real session silently writes a malformed artifact and the commit
    /// gate breaks. Failing here surfaces the drift at build time.
    #[test]
    fn reviewer_md_example_invocations_round_trip() {
        const DOC: &str = include_str!("../../../../.claude/agents/reviewer.md");
        let mut count = 0;
        for line in DOC.lines() {
            for flag in ["--blocker", "--nit"] {
                let Some((_, after)) = line.split_once(flag) else {
                    continue;
                };
                // Only match the quoted `file:line:message` form. Prose
                // mentions in reviewer.md use backticks (`--blocker`) and
                // have no quoted argument, so they are skipped.
                let Some(rest) = after.trim_start().strip_prefix('"') else {
                    continue;
                };
                let Some(end) = rest.find('"') else {
                    continue;
                };
                let spec = &rest[..end];
                let entry = parse_entry(spec).unwrap_or_else(|e| {
                    panic!("reviewer.md example `{flag} \"{spec}\"` failed to parse: {e:#}")
                });
                assert!(!entry.file.is_empty());
                assert!(!entry.message.is_empty());
                count += 1;
            }
        }
        assert!(
            count >= 2,
            "expected at least two quoted example invocations in reviewer.md; found {count} \
             (the example block may have been removed — restore it or relax this lint)"
        );
    }
}
