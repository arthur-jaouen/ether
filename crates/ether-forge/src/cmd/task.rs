//! `ether-forge task T<n> [--context]` — print a task file, optionally
//! appending the matching ROADMAP section as a single blob suitable for
//! feeding to an agent or LLM.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::cmd::get::locate;
use crate::frontmatter::Frontmatter;
use crate::roadmap::{self, Section};
use crate::task::Task;

/// Print a task's raw file. When `context` is true, also append the linked
/// ROADMAP section — matched either by an explicit `roadmap_section:`
/// frontmatter tag or by keyword fallback against the task title and body.
pub fn run(backlog_dir: &Path, roadmap_path: &Path, id: &str, context: bool) -> Result<()> {
    let path = locate(backlog_dir, id)?;
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading task file {}", path.display()))?;

    let blob = render(&raw, &path, roadmap_path, context)?;
    print!("{blob}");
    if !blob.ends_with('\n') {
        println!();
    }
    Ok(())
}

/// Build the output blob — kept separate from `run` so tests can assert on
/// the rendered shape without touching stdout.
fn render(raw: &str, task_path: &Path, roadmap_path: &Path, context: bool) -> Result<String> {
    let mut out = String::from(raw);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if !context {
        return Ok(out);
    }

    let sections = roadmap::parse(roadmap_path)?;
    let task = Task::load(task_path)?;
    let explicit = explicit_section_tag(raw)?;

    let matched = match explicit.as_deref() {
        Some(tag) => roadmap::find_by_title(tag, &sections),
        None => roadmap::best_match_for_task(&task.title, &task.body, &sections),
    };

    out.push('\n');
    out.push_str("## Linked ROADMAP section\n\n");
    match matched {
        Some(sec) => out.push_str(&render_section(sec)),
        None => out.push_str(&render_missing(&explicit, roadmap_path, &sections)),
    }
    Ok(out)
}

fn render_section(sec: &Section) -> String {
    let mut s = format!("### {}\n", sec.title);
    if !sec.body.is_empty() {
        s.push('\n');
        s.push_str(&sec.body);
        if !s.ends_with('\n') {
            s.push('\n');
        }
    }
    s
}

fn render_missing(explicit: &Option<String>, roadmap_path: &Path, sections: &[Section]) -> String {
    if sections.is_empty() {
        return format!(
            "_(no ROADMAP sections parsed from {})_\n",
            roadmap_path.display()
        );
    }
    match explicit {
        Some(tag) => format!("_(no ROADMAP section titled `{tag}`)_\n"),
        None => "_(no ROADMAP section matched this task)_\n".to_string(),
    }
}

/// Read a `roadmap_section: <title>` scalar from the task frontmatter, if
/// present. Missing frontmatter or missing field both yield `Ok(None)`.
fn explicit_section_tag(raw: &str) -> Result<Option<String>> {
    let Some(rest) = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"))
    else {
        return Ok(None);
    };
    let Some(end) = rest.find("\n---\n").or_else(|| rest.find("\n---\r\n")) else {
        return Ok(None);
    };
    let fm = Frontmatter::parse(&rest[..end])?;
    Ok(fm
        .scalar("roadmap_section")
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROADMAP: &str = "# ROADMAP\n\n## Core storage\n\nSparse sets and archetypes.\n\n## Tooling\n\nether-forge backlog CLI and worktree helpers.\n";

    fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let backlog = dir.path().join("backlog");
        fs::create_dir(&backlog).unwrap();
        let roadmap = dir.path().join("ROADMAP.md");
        fs::write(&roadmap, ROADMAP).unwrap();
        (dir, backlog, roadmap)
    }

    #[test]
    fn render_without_context_matches_raw_file() {
        let (_g, backlog, roadmap) = setup();
        let raw = "---\nid: T42\ntitle: demo\nsize: S\nstatus: ready\n---\n\nBody.\n";
        let path = backlog.join("T42-demo.md");
        fs::write(&path, raw).unwrap();
        let out = render(raw, &path, &roadmap, false).unwrap();
        assert_eq!(out, raw);
    }

    #[test]
    fn context_appends_matching_section_via_keyword_fallback() {
        let (_g, backlog, roadmap) = setup();
        let raw = "---\nid: T7\ntitle: ether-forge worktree helper\nsize: S\nstatus: ready\n---\n\nWires up worktree tooling for the backlog CLI.\n";
        let path = backlog.join("T7-worktree.md");
        fs::write(&path, raw).unwrap();
        let out = render(raw, &path, &roadmap, true).unwrap();
        assert!(out.starts_with(raw));
        assert!(out.contains("## Linked ROADMAP section"));
        assert!(out.contains("### Tooling"));
        assert!(out.contains("ether-forge backlog CLI and worktree helpers."));
        assert!(!out.contains("Sparse sets"));
    }

    #[test]
    fn context_honors_explicit_frontmatter_tag() {
        let (_g, backlog, roadmap) = setup();
        // Title would keyword-match "Tooling" but the explicit tag overrides.
        let raw = "---\nid: T8\ntitle: ether-forge backlog helper\nsize: S\nstatus: ready\nroadmap_section: Core storage\n---\n\nBody.\n";
        let path = backlog.join("T8-demo.md");
        fs::write(&path, raw).unwrap();
        let out = render(raw, &path, &roadmap, true).unwrap();
        assert!(out.contains("### Core storage"));
        assert!(out.contains("Sparse sets and archetypes"));
        assert!(!out.contains("### Tooling"));
    }

    #[test]
    fn context_reports_missing_section_when_no_match() {
        let (_g, backlog, roadmap) = setup();
        let raw =
            "---\nid: T9\ntitle: totally unrelated topic\nsize: S\nstatus: ready\n---\n\nbody\n";
        let path = backlog.join("T9-x.md");
        fs::write(&path, raw).unwrap();
        let out = render(raw, &path, &roadmap, true).unwrap();
        assert!(out.contains("no ROADMAP section matched"));
    }

    #[test]
    fn context_reports_empty_roadmap_gracefully() {
        let (guard, backlog, _roadmap) = setup();
        let missing = guard.path().join("MISSING.md");
        let raw = "---\nid: T1\ntitle: any\nsize: S\nstatus: ready\n---\n\nbody\n";
        let path = backlog.join("T1-x.md");
        fs::write(&path, raw).unwrap();
        let out = render(raw, &path, &missing, true).unwrap();
        assert!(out.contains("no ROADMAP sections parsed"));
    }
}
