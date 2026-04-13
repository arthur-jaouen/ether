//! `ether-forge validate` — integrity lint across the backlog.
//!
//! Two modes:
//!
//! * **Default**: walks `backlog/` + `backlog/done/` and reports duplicate
//!   ids, malformed filenames, bad `depends_on` refs, cycles, etc.
//! * **`--diff-only [T<n>]`**: skips backlog work entirely and runs the
//!   reviewer-subset source scans from [`crate::cmd::diff_scan`] over
//!   `git diff main`. The optional task id points at the task's worktree
//!   so the same checks can run inside an in-flight skill branch.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cmd::{diff, diff_scan};
use crate::task::{Status, Task};

/// Run validation over `backlog_dir` (active) and `backlog_dir/done`.
///
/// Prints a grouped error report to stderr on failure and returns an error;
/// prints `OK` to stdout and returns `Ok(())` on a clean backlog.
pub fn run(backlog_dir: &Path) -> Result<()> {
    let report = validate(backlog_dir)?;
    if report.is_empty() {
        println!("OK");
        Ok(())
    } else {
        eprintln!("{}", report.render());
        anyhow::bail!("{} validation error(s)", report.errors.len());
    }
}

/// Run the reviewer-subset source scans over `git diff main`.
///
/// When `task_id` is `Some`, the diff is captured from the task's worktree
/// under `worktrees/<id>-<slug>/`; otherwise from the repo root. Emits
/// findings grouped by check category in deterministic order.
pub fn run_diff_only(backlog_dir: &Path, task_id: Option<&str>) -> Result<()> {
    let findings = scan_diff_only(backlog_dir, task_id)?;
    if findings.is_empty() {
        println!("OK");
        return Ok(());
    }
    eprintln!("{}", render_diff_findings(&findings));
    anyhow::bail!("{} diff-scan finding(s)", findings.len());
}

/// Capture the diff for the target work dir and run every scan over it.
///
/// Extracted from [`run_diff_only`] so tests can assert on the structured
/// findings list without going through stdout/stderr.
pub fn scan_diff_only(
    backlog_dir: &Path,
    task_id: Option<&str>,
) -> Result<Vec<diff_scan::ScanFinding>> {
    let work_dir = diff::resolve_work_dir(backlog_dir, task_id)?;
    let raw = diff::git_diff_main(&work_dir)?;
    let files = diff_scan::parse_diff(&raw);
    let mut out = Vec::new();
    for file in &files {
        out.extend(diff_scan::scan_file(file));
    }
    // Deterministic ordering: (check, file, lineno) — matches the category
    // grouping that `render_diff_findings` prints.
    out.sort_by(|a, b| (a.check, &a.file, a.lineno).cmp(&(b.check, &b.file, b.lineno)));
    Ok(out)
}

/// Render diff-scan findings grouped by check category, one bullet per
/// finding, in deterministic order. Shape mirrors [`Report::render`] so
/// downstream consumers don't need to distinguish the two modes visually.
pub fn render_diff_findings(findings: &[diff_scan::ScanFinding]) -> String {
    let mut grouped: BTreeMap<diff_scan::Check, Vec<&diff_scan::ScanFinding>> = BTreeMap::new();
    for f in findings {
        grouped.entry(f.check).or_default().push(f);
    }
    let mut out = String::new();
    for (check, entries) in grouped {
        out.push_str(&format!("{}:\n", check.label()));
        for f in entries {
            out.push_str(&format!("  - {}:{} — {}\n", f.file, f.lineno, f.message));
        }
    }
    out
}

/// A single validation failure, grouped by category for reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub category: Category,
    pub message: String,
}

/// Validation failure categories. Sorted for deterministic output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    DuplicateId,
    Filename,
    DependsOn,
    BlockedConsistency,
    CommitField,
}

impl Category {
    fn label(&self) -> &'static str {
        match self {
            Category::DuplicateId => "duplicate ids",
            Category::Filename => "filename",
            Category::DependsOn => "depends_on",
            Category::BlockedConsistency => "blocked/depends_on consistency",
            Category::CommitField => "commit field",
        }
    }
}

/// Aggregated validation report.
#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<Finding>,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Render findings grouped by category in deterministic order.
    pub fn render(&self) -> String {
        let mut grouped: BTreeMap<Category, Vec<&str>> = BTreeMap::new();
        for f in &self.errors {
            grouped.entry(f.category).or_default().push(&f.message);
        }
        let mut out = String::new();
        for (cat, msgs) in grouped {
            out.push_str(&format!("{}:\n", cat.label()));
            for m in msgs {
                out.push_str(&format!("  - {m}\n"));
            }
        }
        out
    }

    fn push(&mut self, category: Category, message: impl Into<String>) {
        self.errors.push(Finding {
            category,
            message: message.into(),
        });
    }
}

/// Load active + done tasks and collect every integrity failure.
pub fn validate(backlog_dir: &Path) -> Result<Report> {
    let mut report = Report::default();

    let active = load_with_paths(backlog_dir)?;
    let done_dir = backlog_dir.join("done");
    let done = if done_dir.exists() {
        load_with_paths(&done_dir)?
    } else {
        Vec::new()
    };

    check_filenames(&active, &mut report);
    check_filenames(&done, &mut report);

    check_duplicate_ids(&active, &done, &mut report);

    for (task, _path) in &active {
        check_active_task(task, &mut report);
    }
    for (task, _path) in &done {
        check_done_task(task, &mut report);
    }

    // depends_on existence + cycles operate on active tasks (done tasks have
    // depends_on stripped by `done` and aren't part of the ready graph).
    let mut id_set: BTreeSet<String> = BTreeSet::new();
    for (t, _) in &active {
        id_set.insert(t.id.clone());
    }
    for (t, _) in &done {
        id_set.insert(t.id.clone());
    }
    check_depends_on_refs(&active, &id_set, &mut report);
    check_cycles(&active, &mut report);

    Ok(report)
}

fn load_with_paths(dir: &Path) -> Result<Vec<(Task, PathBuf)>> {
    let tasks =
        Task::load_all(dir).with_context(|| format!("loading tasks from {}", dir.display()))?;
    // Re-derive paths: Task::load_all doesn't expose them, so walk again.
    let mut out = Vec::with_capacity(tasks.len());
    for t in tasks {
        let path = dir.join(expected_filename(&t.id, &t.title));
        out.push((t, path));
    }
    Ok(out)
}

fn expected_filename(id: &str, title: &str) -> String {
    format!("{id}-{}.md", slugify(title))
}

fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut last_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn check_filenames(tasks: &[(Task, PathBuf)], report: &mut Report) {
    for (task, _expected_path) in tasks {
        // Walk the real directory to find the actual file name for this task.
        // `load_with_paths` synthesized `_expected_path`, but we need to compare
        // against the real file name instead — locate it by ID prefix.
        let parent = match _expected_path.parent() {
            Some(p) => p,
            None => continue,
        };
        let prefix = format!("{}-", task.id);
        let real = match std::fs::read_dir(parent) {
            Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.file_name()).find(|n| {
                n.to_string_lossy().starts_with(&prefix) && n.to_string_lossy().ends_with(".md")
            }),
            Err(_) => None,
        };
        let Some(real) = real else {
            continue;
        };
        let real_name = real.to_string_lossy().into_owned();
        if !real_name.starts_with(&prefix) || !real_name.ends_with(".md") {
            report.push(
                Category::Filename,
                format!(
                    "{}: `{real_name}` does not match `T<id>-<slug>.md`",
                    task.id
                ),
            );
            continue;
        }
        let middle = &real_name[prefix.len()..real_name.len() - 3];
        if middle.is_empty() || middle != slugify(middle) {
            report.push(
                Category::Filename,
                format!(
                    "{}: `{real_name}` slug is not lowercase-alphanumeric-hyphens",
                    task.id
                ),
            );
        }
    }
}

fn check_duplicate_ids(active: &[(Task, PathBuf)], done: &[(Task, PathBuf)], report: &mut Report) {
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    for (t, _) in active.iter().chain(done.iter()) {
        *seen.entry(t.id.clone()).or_insert(0) += 1;
    }
    for (id, count) in seen {
        if count > 1 {
            report.push(
                Category::DuplicateId,
                format!("{id}: appears {count} times across active + done"),
            );
        }
    }
}

fn check_active_task(task: &Task, report: &mut Report) {
    match task.status {
        Status::Blocked => {
            if task.depends_on.is_empty() {
                report.push(
                    Category::BlockedConsistency,
                    format!("{}: status=blocked but depends_on is empty", task.id),
                );
            }
        }
        Status::Ready | Status::Draft => {
            if !task.depends_on.is_empty() {
                report.push(
                    Category::BlockedConsistency,
                    format!(
                        "{}: depends_on set but status is {:?} (should be blocked)",
                        task.id, task.status
                    ),
                );
            }
        }
        Status::Done => {
            report.push(
                Category::BlockedConsistency,
                format!("{}: status=done but file lives in active backlog", task.id),
            );
        }
    }
    if task.commit.is_some() {
        report.push(
            Category::CommitField,
            format!("{}: active task has a `commit` field", task.id),
        );
    }
}

fn check_done_task(task: &Task, report: &mut Report) {
    if task.status != Status::Done {
        report.push(
            Category::BlockedConsistency,
            format!("{}: file in done/ but status is {:?}", task.id, task.status),
        );
    }
    if task.commit.is_none() {
        report.push(
            Category::CommitField,
            format!("{}: done task is missing a `commit` field", task.id),
        );
    }
    if !task.depends_on.is_empty() {
        report.push(
            Category::DependsOn,
            format!("{}: done task still has depends_on entries", task.id),
        );
    }
}

fn check_depends_on_refs(
    active: &[(Task, PathBuf)],
    known: &BTreeSet<String>,
    report: &mut Report,
) {
    for (task, _) in active {
        for dep in &task.depends_on {
            if dep == &task.id {
                report.push(Category::DependsOn, format!("{}: self-dependency", task.id));
                continue;
            }
            if !known.contains(dep) {
                report.push(
                    Category::DependsOn,
                    format!("{}: depends on unknown task {dep}", task.id),
                );
            }
        }
    }
}

fn check_cycles(active: &[(Task, PathBuf)], report: &mut Report) {
    // Build adjacency from active tasks only (done tasks are terminal).
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (t, _) in active {
        graph.insert(t.id.clone(), t.depends_on.clone());
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        White,
        Gray,
        Black,
    }
    let mut marks: BTreeMap<String, Mark> =
        graph.keys().map(|k| (k.clone(), Mark::White)).collect();
    let mut reported: BTreeSet<Vec<String>> = BTreeSet::new();

    fn dfs(
        node: &str,
        graph: &BTreeMap<String, Vec<String>>,
        marks: &mut BTreeMap<String, Mark>,
        stack: &mut Vec<String>,
        reported: &mut BTreeSet<Vec<String>>,
        report: &mut Report,
    ) {
        marks.insert(node.to_string(), Mark::Gray);
        stack.push(node.to_string());
        if let Some(neighbors) = graph.get(node) {
            for next in neighbors {
                match marks.get(next).copied().unwrap_or(Mark::Black) {
                    Mark::White => dfs(next, graph, marks, stack, reported, report),
                    Mark::Gray => {
                        if let Some(pos) = stack.iter().position(|n| n == next) {
                            let mut cycle: Vec<String> = stack[pos..].to_vec();
                            cycle.push(next.clone());
                            let mut key = cycle.clone();
                            key.sort();
                            if reported.insert(key) {
                                report.push(
                                    Category::DependsOn,
                                    format!("cycle: {}", cycle.join(" -> ")),
                                );
                            }
                        }
                    }
                    Mark::Black => {}
                }
            }
        }
        stack.pop();
        marks.insert(node.to_string(), Mark::Black);
    }

    let ids: Vec<String> = graph.keys().cloned().collect();
    for id in ids {
        if marks.get(&id).copied() == Some(Mark::White) {
            let mut stack = Vec::new();
            dfs(&id, &graph, &mut marks, &mut stack, &mut reported, report);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::diff_scan::{Check, ScanFinding};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn render_diff_findings_groups_by_check_in_stable_order() {
        let findings = vec![
            ScanFinding {
                check: Check::TodoMarker,
                file: "a.rs".into(),
                lineno: 5,
                message: "new `TODO` marker in added line".into(),
            },
            ScanFinding {
                check: Check::UnsafeWithoutSafety,
                file: "b.rs".into(),
                lineno: 10,
                message: "new `unsafe` without a `// SAFETY:` comment on the preceding lines"
                    .into(),
            },
            ScanFinding {
                check: Check::NonDeterministicIter,
                file: "c.rs".into(),
                lineno: 3,
                message: "new `HashMap` reference".into(),
            },
        ];
        let out = render_diff_findings(&findings);
        // Enum ordering: Unsafe < NonDeterministicIter < TodoMarker.
        let unsafe_idx = out.find("unsafe without SAFETY").unwrap();
        let iter_idx = out.find("non-deterministic HashMap").unwrap();
        let todo_idx = out.find("TODO/FIXME marker").unwrap();
        assert!(unsafe_idx < iter_idx);
        assert!(iter_idx < todo_idx);
        // Each bullet carries `file:lineno — message`.
        assert!(out.contains("  - a.rs:5 — new `TODO`"));
        assert!(out.contains("  - b.rs:10 — new `unsafe`"));
        assert!(out.contains("  - c.rs:3 — new `HashMap`"));
    }

    #[test]
    fn render_diff_findings_empty_is_empty_string() {
        assert_eq!(render_diff_findings(&[]), "");
    }

    fn write(dir: &Path, name: &str, body: &str) {
        fs::write(dir.join(name), body).unwrap();
    }

    fn fixture() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("done")).unwrap();
        dir
    }

    fn active_task(id: &str, title: &str, status: &str, extras: &str) -> String {
        format!("---\nid: {id}\ntitle: {title}\nsize: S\nstatus: {status}\n{extras}---\n\nbody\n")
    }

    #[test]
    fn clean_backlog_is_ok() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "ready", ""),
        );
        write(
            dir.path(),
            "T2-beta.md",
            &active_task("T2", "beta", "blocked", "depends_on:\n  - T1\n"),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report.is_empty(), "unexpected: {}", report.render());
    }

    #[test]
    fn detects_duplicate_ids() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "ready", ""),
        );
        write(
            dir.path().join("done").as_path(),
            "T1-alpha.md",
            "---\nid: T1\ntitle: alpha\nsize: S\nstatus: done\ncommit: abc1234\n---\n\n",
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.category == Category::DuplicateId));
    }

    #[test]
    fn detects_unknown_depends_on() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "blocked", "depends_on:\n  - T99\n"),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.message.contains("unknown task T99")));
    }

    #[test]
    fn detects_self_dependency() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "blocked", "depends_on:\n  - T1\n"),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.message.contains("self-dependency")));
    }

    #[test]
    fn detects_cycle() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "blocked", "depends_on:\n  - T2\n"),
        );
        write(
            dir.path(),
            "T2-beta.md",
            &active_task("T2", "beta", "blocked", "depends_on:\n  - T1\n"),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.message.starts_with("cycle:")));
    }

    #[test]
    fn detects_blocked_without_depends_on() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "blocked", ""),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.category == Category::BlockedConsistency));
    }

    #[test]
    fn detects_ready_with_depends_on() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "ready", "depends_on:\n  - T2\n"),
        );
        write(
            dir.path(),
            "T2-beta.md",
            &active_task("T2", "beta", "ready", ""),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.category == Category::BlockedConsistency));
    }

    #[test]
    fn detects_done_missing_commit() {
        let dir = fixture();
        write(
            dir.path().join("done").as_path(),
            "T1-alpha.md",
            "---\nid: T1\ntitle: alpha\nsize: S\nstatus: done\n---\n\n",
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.category == Category::CommitField));
    }

    #[test]
    fn detects_active_with_commit() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-alpha.md",
            &active_task("T1", "alpha", "ready", "commit: abc1234\n"),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.category == Category::CommitField));
    }

    #[test]
    fn detects_bad_filename() {
        let dir = fixture();
        write(
            dir.path(),
            "T1-Bad_Name.md",
            &active_task("T1", "alpha", "ready", ""),
        );
        let report = validate(dir.path()).unwrap();
        assert!(report
            .errors
            .iter()
            .any(|f| f.category == Category::Filename));
    }
}
