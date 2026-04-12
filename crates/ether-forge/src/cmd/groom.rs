//! `ether-forge groom` — audit coverage, lint integrity, flag drift.
//!
//! Dry-run by default. Mutates the backlog only with `--apply` and only for
//! cascade fix-ups (removing `depends_on` entries pointing at tasks already in
//! `done/`). Lint errors, coverage gaps, size mismatches, and stale sub-step
//! references are reported but never auto-fixed.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cmd::validate::{validate, Finding};
use crate::frontmatter::Frontmatter;
use crate::task::{Size, Status, Task};

/// Run the groom audit. `apply` enables cascade mutation; `json` switches
/// output mode. `roadmap_path` may point at a missing file — coverage is then
/// reported as empty instead of erroring.
pub fn run(backlog_dir: &Path, roadmap_path: &Path, apply: bool, json: bool) -> Result<()> {
    let report = audit(backlog_dir, roadmap_path)?;

    if apply {
        apply_cascades(backlog_dir, &report.cascades)?;
    }

    if json {
        let out = serde_json::to_string_pretty(&report)?;
        println!("{out}");
    } else {
        print!("{}", render_human(&report, apply));
    }
    Ok(())
}

/// Full groom report, serializable for `--json` and renderable for humans.
#[derive(Debug, Serialize)]
pub struct GroomReport {
    pub lint: Vec<LintEntry>,
    pub coverage: Vec<CoverageEntry>,
    pub flags: Vec<Flag>,
    pub cascades: Vec<Cascade>,
}

#[derive(Debug, Serialize)]
pub struct LintEntry {
    pub category: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CoverageEntry {
    pub section: String,
    pub classification: Classification,
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Classification {
    /// At least one active task covers the section.
    Covered,
    /// Mix of active + done tasks — section has ongoing work.
    Partial,
    /// No active or done task references the section.
    Uncovered,
    /// All matching tasks are done.
    Done,
}

#[derive(Debug, Serialize)]
pub struct Flag {
    pub kind: FlagKind,
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagKind {
    SizeMismatch,
    StaleSubStep,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cascade {
    pub task_id: String,
    pub removed_dep: String,
    pub becomes_ready: bool,
}

fn audit(backlog_dir: &Path, roadmap_path: &Path) -> Result<GroomReport> {
    let lint = validate(backlog_dir)?
        .errors
        .into_iter()
        .map(|f: Finding| LintEntry {
            category: format!("{:?}", f.category),
            message: f.message,
        })
        .collect();

    let active = Task::load_all(backlog_dir)
        .with_context(|| format!("loading active backlog from {}", backlog_dir.display()))?;
    let done_dir = backlog_dir.join("done");
    let done = if done_dir.exists() {
        Task::load_all(&done_dir)
            .with_context(|| format!("loading done backlog from {}", done_dir.display()))?
    } else {
        Vec::new()
    };

    let sections = parse_roadmap(roadmap_path)?;
    let coverage = classify_sections(&sections, &active, &done);
    let mut flags = Vec::new();
    for task in &active {
        flags.extend(flag_size_mismatch(task));
        flags.extend(flag_stale_sub_steps(task, backlog_dir));
    }
    flags.sort_by(|a, b| a.task_id.cmp(&b.task_id).then(a.message.cmp(&b.message)));

    let cascades = compute_cascades(&active, &done);

    Ok(GroomReport {
        lint,
        coverage,
        flags,
        cascades,
    })
}

// ---------------------------------------------------------------------------
// ROADMAP parsing
// ---------------------------------------------------------------------------

/// A parsed ROADMAP heading used as a coverage unit.
#[derive(Debug, Clone)]
struct Section {
    /// Heading text without leading hashes.
    title: String,
    /// Lowercased keywords extracted from the title (>=4 chars, alphanumeric).
    keywords: Vec<String>,
}

fn parse_roadmap(path: &Path) -> Result<Vec<Section>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading roadmap {}", path.display()))?;
    let mut sections = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim_start();
        // Only consider level-2 and level-3 headings — level-1 is the file title.
        let title = if let Some(rest) = trimmed.strip_prefix("### ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            rest
        } else {
            continue;
        };
        let keywords = extract_keywords(title);
        if keywords.is_empty() {
            continue;
        }
        sections.push(Section {
            title: title.trim().to_string(),
            keywords,
        });
    }
    Ok(sections)
}

fn extract_keywords(text: &str) -> Vec<String> {
    // Short words and common glue carry no signal for section→task matching.
    const STOP: &[&str] = &[
        "the",
        "and",
        "for",
        "with",
        "from",
        "into",
        "that",
        "this",
        "than",
        "then",
        "phase",
        "goal",
        "when",
        "what",
        "will",
        "over",
        "also",
        "only",
        "non-goals",
        "non",
    ];
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .filter_map(|w| {
            let w = w.trim_matches('-').to_ascii_lowercase();
            if w.len() < 4 || STOP.contains(&w.as_str()) {
                None
            } else {
                Some(w)
            }
        })
        .collect()
}

fn classify_sections(sections: &[Section], active: &[Task], done: &[Task]) -> Vec<CoverageEntry> {
    let mut out: Vec<CoverageEntry> = sections
        .iter()
        .map(|s| {
            let active_hits = match_tasks(s, active);
            let done_hits = match_tasks(s, done);
            let classification = match (active_hits.is_empty(), done_hits.is_empty()) {
                (true, true) => Classification::Uncovered,
                (true, false) => Classification::Done,
                (false, true) => Classification::Covered,
                (false, false) => Classification::Partial,
            };
            let mut task_ids: Vec<String> = active_hits
                .iter()
                .chain(done_hits.iter())
                .map(|t| t.id.clone())
                .collect();
            task_ids.sort();
            task_ids.dedup();
            CoverageEntry {
                section: s.title.clone(),
                classification,
                task_ids,
            }
        })
        .collect();
    out.sort_by(|a, b| a.section.cmp(&b.section));
    out
}

fn match_tasks<'a>(section: &Section, tasks: &'a [Task]) -> Vec<&'a Task> {
    let mut hits = Vec::new();
    for t in tasks {
        let haystack = format!("{} {}", t.title, t.body).to_ascii_lowercase();
        // Require at least two keyword hits to filter accidental one-word matches
        // (e.g. "backlog" appearing in every task).
        let matches = section
            .keywords
            .iter()
            .filter(|k| haystack.contains(k.as_str()))
            .count();
        let threshold = if section.keywords.len() >= 2 { 2 } else { 1 };
        if matches >= threshold {
            hits.push(t);
        }
    }
    hits
}

// ---------------------------------------------------------------------------
// Flag: size vs sub-step count mismatch
// ---------------------------------------------------------------------------

fn flag_size_mismatch(task: &Task) -> Vec<Flag> {
    let count = count_sub_steps(&task.body);
    if count == 0 {
        return Vec::new();
    }
    let ok = match task.size {
        Size::S => (1..=3).contains(&count),
        Size::M => (3..=6).contains(&count),
        Size::L => count >= 6,
    };
    if ok {
        Vec::new()
    } else {
        vec![Flag {
            kind: FlagKind::SizeMismatch,
            task_id: task.id.clone(),
            message: format!(
                "size {} expects {} sub-steps but body has {}",
                task.size.as_str(),
                size_hint(task.size),
                count
            ),
        }]
    }
}

fn size_hint(size: Size) -> &'static str {
    match size {
        Size::S => "1-3",
        Size::M => "3-6",
        Size::L => "6+",
    }
}

fn count_sub_steps(body: &str) -> usize {
    body.lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("- [ ]") || t.starts_with("- [x]") || t.starts_with("- [X]")
        })
        .count()
}

// ---------------------------------------------------------------------------
// Flag: stale sub-step references (paths that no longer exist)
// ---------------------------------------------------------------------------

fn flag_stale_sub_steps(task: &Task, backlog_dir: &Path) -> Vec<Flag> {
    let mut flags = Vec::new();
    // Workspace root is the backlog's parent — ether-forge commands always run
    // from the workspace root in practice.
    let workspace = backlog_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for line in task.body.lines() {
        let t = line.trim_start();
        if !(t.starts_with("- [ ]") || t.starts_with("- [x]") || t.starts_with("- [X]")) {
            continue;
        }
        for path in extract_backticked_paths(t) {
            if !seen.insert(path.clone()) {
                continue;
            }
            // Only check things that look like workspace-relative paths.
            if !path.contains('/') && !path.ends_with(".rs") && !path.ends_with(".md") {
                continue;
            }
            let full = workspace.join(&path);
            if !full.exists() {
                flags.push(Flag {
                    kind: FlagKind::StaleSubStep,
                    task_id: task.id.clone(),
                    message: format!("references missing path `{path}`"),
                });
            }
        }
    }
    flags
}

fn extract_backticked_paths(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'`' {
                j += 1;
            }
            if j < bytes.len() {
                let content = &line[start..j];
                // Only flag things that plausibly name a file on disk.
                if looks_like_path(content) {
                    out.push(content.to_string());
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn looks_like_path(s: &str) -> bool {
    if s.is_empty() || s.contains(' ') {
        return false;
    }
    // Must contain a slash or a file extension to count.
    s.contains('/') && (s.contains('.') || s.ends_with('/'))
}

// ---------------------------------------------------------------------------
// Cascade: tasks whose depends_on references already-done tasks
// ---------------------------------------------------------------------------

fn compute_cascades(active: &[Task], done: &[Task]) -> Vec<Cascade> {
    let done_ids: BTreeSet<String> = done.iter().map(|t| t.id.clone()).collect();
    let mut out = Vec::new();
    // Track per-task remaining count so we can predict `becomes_ready`.
    let mut remaining: BTreeMap<String, usize> = active
        .iter()
        .map(|t| (t.id.clone(), t.depends_on.len()))
        .collect();
    for t in active {
        for dep in &t.depends_on {
            if done_ids.contains(dep) {
                let r = remaining.get_mut(&t.id).unwrap();
                *r -= 1;
                out.push(Cascade {
                    task_id: t.id.clone(),
                    removed_dep: dep.clone(),
                    becomes_ready: *r == 0 && t.status == Status::Blocked,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        a.task_id
            .cmp(&b.task_id)
            .then(a.removed_dep.cmp(&b.removed_dep))
    });
    out
}

fn apply_cascades(backlog_dir: &Path, cascades: &[Cascade]) -> Result<()> {
    // Group cascades by task id so we rewrite each file once.
    let mut by_task: BTreeMap<String, Vec<&Cascade>> = BTreeMap::new();
    for c in cascades {
        by_task.entry(c.task_id.clone()).or_default().push(c);
    }
    for (id, entries) in by_task {
        let path = find_active_file(backlog_dir, &id)?;
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let (fm_text, body) = split_frontmatter(&raw)
            .with_context(|| format!("parsing frontmatter in {}", path.display()))?;
        let mut fm = Frontmatter::parse(fm_text)?;
        let mut now_empty = false;
        for c in entries {
            if fm.list_items("depends_on").contains(&c.removed_dep) {
                now_empty = fm.remove_list_item("depends_on", &c.removed_dep)?;
            }
        }
        if now_empty {
            fm.remove("depends_on");
            if fm.scalar("status") == Some("blocked") {
                fm.set_scalar("status", "ready");
            }
        }
        let new_raw = render(&fm, body);
        fs::write(&path, new_raw).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

fn find_active_file(backlog_dir: &Path, id: &str) -> Result<PathBuf> {
    let prefix = format!("{id}-");
    for entry in fs::read_dir(backlog_dir)
        .with_context(|| format!("reading backlog dir {}", backlog_dir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&prefix) && name.ends_with(".md") {
            return Ok(entry.path());
        }
    }
    anyhow::bail!("no active task file for {id}")
}

fn split_frontmatter(raw: &str) -> Result<(&str, &str)> {
    let rest = raw
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow::anyhow!("missing opening `---` fence"))?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| anyhow::anyhow!("missing closing `---` fence"))?;
    let frontmatter = &rest[..end];
    let after = &rest[end + 1..];
    let body = after
        .strip_prefix("---\n")
        .unwrap_or(after)
        .trim_start_matches('\n');
    Ok((frontmatter, body))
}

fn render(fm: &Frontmatter, body: &str) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&fm.to_string());
    out.push_str("\n---\n");
    if !body.is_empty() {
        out.push('\n');
        out.push_str(body);
    }
    out
}

// ---------------------------------------------------------------------------
// Human rendering
// ---------------------------------------------------------------------------

fn render_human(report: &GroomReport, applied: bool) -> String {
    let mut out = String::new();
    out.push_str("# groom report\n\n");

    out.push_str(&format!("## lint ({})\n", report.lint.len()));
    if report.lint.is_empty() {
        out.push_str("  clean\n");
    } else {
        for l in &report.lint {
            out.push_str(&format!("  [{}] {}\n", l.category, l.message));
        }
    }

    out.push_str(&format!("\n## coverage ({})\n", report.coverage.len()));
    if report.coverage.is_empty() {
        out.push_str("  (no ROADMAP sections parsed)\n");
    } else {
        for c in &report.coverage {
            let tag = match c.classification {
                Classification::Covered => "covered",
                Classification::Partial => "partial",
                Classification::Uncovered => "UNCOVERED",
                Classification::Done => "done",
            };
            let ids = if c.task_ids.is_empty() {
                "-".to_string()
            } else {
                c.task_ids.join(", ")
            };
            out.push_str(&format!("  [{tag}] {} — {ids}\n", c.section));
        }
    }

    out.push_str(&format!("\n## flags ({})\n", report.flags.len()));
    if report.flags.is_empty() {
        out.push_str("  clean\n");
    } else {
        for f in &report.flags {
            out.push_str(&format!(
                "  {}: {} — {}\n",
                kind_label(f.kind),
                f.task_id,
                f.message
            ));
        }
    }

    let verb = if applied { "applied" } else { "pending" };
    out.push_str(&format!(
        "\n## cascades ({}, {verb})\n",
        report.cascades.len()
    ));
    if report.cascades.is_empty() {
        out.push_str("  clean\n");
    } else {
        for c in &report.cascades {
            let ready = if c.becomes_ready { " → ready" } else { "" };
            out.push_str(&format!(
                "  {}: drop {}{}\n",
                c.task_id, c.removed_dep, ready
            ));
        }
    }
    out
}

fn kind_label(k: FlagKind) -> &'static str {
    match k {
        FlagKind::SizeMismatch => "size",
        FlagKind::StaleSubStep => "stale",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_drop_stopwords_and_shorts() {
        let k = extract_keywords("Phase 0 — ether-forge (active focus)");
        assert!(k.contains(&"ether-forge".to_string()));
        assert!(k.contains(&"active".to_string()));
        assert!(!k.iter().any(|w| w == "phase"));
    }

    #[test]
    fn sub_step_counter_handles_both_states() {
        let body = "## Sub-steps\n\n- [ ] one\n- [x] two\n- [X] three\nfoo\n";
        assert_eq!(count_sub_steps(body), 3);
    }

    #[test]
    fn size_mismatch_flags_small_with_seven_steps() {
        let task = mk_task("T1", "t", Size::S, Status::Ready, &[], seven_steps());
        let flags = flag_size_mismatch(&task);
        assert_eq!(flags.len(), 1);
        assert!(flags[0].message.contains("1-3"));
    }

    #[test]
    fn size_mismatch_quiet_when_in_range() {
        let task = mk_task(
            "T1",
            "t",
            Size::M,
            Status::Ready,
            &[],
            "- [ ] a\n- [ ] b\n- [ ] c\n- [ ] d\n",
        );
        assert!(flag_size_mismatch(&task).is_empty());
    }

    #[test]
    fn extract_backticked_paths_finds_path_like() {
        let paths =
            extract_backticked_paths("- [ ] edit `crates/ether-forge/src/foo.rs` and `not a path`");
        assert_eq!(paths, vec!["crates/ether-forge/src/foo.rs".to_string()]);
    }

    #[test]
    fn cascade_identifies_done_deps() {
        let active = vec![mk_task("T2", "t", Size::S, Status::Blocked, &["T1"], "")];
        let done = vec![mk_task("T1", "t", Size::S, Status::Done, &[], "")];
        let cascades = compute_cascades(&active, &done);
        assert_eq!(cascades.len(), 1);
        assert_eq!(cascades[0].task_id, "T2");
        assert_eq!(cascades[0].removed_dep, "T1");
        assert!(cascades[0].becomes_ready);
    }

    #[test]
    fn cascade_partial_keeps_blocked() {
        let active = vec![mk_task(
            "T3",
            "t",
            Size::S,
            Status::Blocked,
            &["T1", "T2"],
            "",
        )];
        let done = vec![mk_task("T1", "t", Size::S, Status::Done, &[], "")];
        let cascades = compute_cascades(&active, &done);
        assert_eq!(cascades.len(), 1);
        assert!(!cascades[0].becomes_ready);
    }

    #[test]
    fn classify_sections_marks_uncovered() {
        let sections = vec![Section {
            title: "Unique nebula subsystem".to_string(),
            keywords: vec!["nebula".into(), "subsystem".into()],
        }];
        let coverage = classify_sections(&sections, &[], &[]);
        assert_eq!(coverage[0].classification, Classification::Uncovered);
    }

    #[test]
    fn classify_sections_marks_covered_on_match() {
        let sections = vec![Section {
            title: "World and Entity".to_string(),
            keywords: vec!["world".into(), "entity".into()],
        }];
        let active = vec![mk_task(
            "T1",
            "World Entity scaffold",
            Size::M,
            Status::Ready,
            &[],
            "Implements world and entity types.\n",
        )];
        let coverage = classify_sections(&sections, &active, &[]);
        assert_eq!(coverage[0].classification, Classification::Covered);
        assert_eq!(coverage[0].task_ids, vec!["T1".to_string()]);
    }

    fn mk_task(
        id: &str,
        title: &str,
        size: Size,
        status: Status,
        deps: &[&str],
        body: &str,
    ) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            size,
            status,
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            priority: None,
            commit: None,
            body: body.to_string(),
        }
    }

    fn seven_steps() -> &'static str {
        "- [ ] a\n- [ ] b\n- [ ] c\n- [ ] d\n- [ ] e\n- [ ] f\n- [ ] g\n"
    }
}
