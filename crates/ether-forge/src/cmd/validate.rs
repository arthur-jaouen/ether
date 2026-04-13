//! `ether-forge validate` — integrity lint across the backlog.
//!
//! Two modes:
//! - default: schema-lint the backlog directory.
//! - `--diff-only`: scope code-review checks (SAFETY on new unsafe blocks,
//!   new `HashMap`/`HashSet` mentions, new `TODO`/`FIXME` markers) to files
//!   touched by `git diff main`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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
    use std::fs;
    use tempfile::TempDir;

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

// =============================================================================
// `--diff-only` mode: code-review checks scoped to `git diff main`.
// =============================================================================

/// Run validate in diff-only mode: scope checks to files touched by
/// `git diff main` (or the task-scoped worktree diff when `task_id` is given).
///
/// Prints one finding per line to stderr and returns an error on any hit;
/// prints `OK` and returns `Ok(())` on a clean diff.
pub fn run_diff_only(backlog_dir: &Path, task_id: Option<&str>) -> Result<()> {
    let work_dir = crate::cmd::diff::resolve_work_dir(backlog_dir, task_id)?;
    let raw = crate::cmd::diff::git_diff_main(&work_dir)?;
    let diff = crate::cmd::diff::filter_lockfiles(&raw);
    let files = parse_diff(&diff);
    let findings = diff_checks(&files, &work_dir);

    if findings.is_empty() {
        println!("OK");
        Ok(())
    } else {
        for f in &findings {
            eprintln!("{f}");
        }
        anyhow::bail!("{} diff finding(s)", findings.len());
    }
}

/// A parsed file section from a unified diff — new-file path plus added lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiffFile {
    /// Path on the new side (`b/<path>`), relative to the repo root.
    pub path: String,
    /// Lines added by this diff — each with its line number in the new file.
    pub added: Vec<AddedLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddedLine {
    /// 1-based line number in the new file.
    pub lineno: u32,
    /// Line text (without the leading `+`, without a trailing newline).
    pub text: String,
}

/// Parse a unified-diff string into per-file lists of added lines.
///
/// Deleted-file sections (`+++ /dev/null`) and binary sections are skipped;
/// hunk headers drive the new-file line counter for every `+` line. Lines
/// starting with `\` (e.g. `\ No newline at end of file`) are ignored.
pub(crate) fn parse_diff(diff: &str) -> Vec<DiffFile> {
    let mut out: Vec<DiffFile> = Vec::new();
    let mut current: Option<DiffFile> = None;
    let mut new_line: u32 = 0;
    let mut in_hunk = false;
    let mut skip_section = false;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            // Flush previous file.
            if let Some(f) = current.take() {
                if !skip_section {
                    out.push(f);
                }
            }
            // Seed a placeholder using the `b/` path from the header; it may
            // be overwritten by a later `+++ b/<path>` line.
            let path = extract_b_path(rest).unwrap_or_default();
            current = Some(DiffFile {
                path,
                added: Vec::new(),
            });
            new_line = 0;
            in_hunk = false;
            skip_section = false;
            continue;
        }

        if skip_section || current.is_none() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("+++ ") {
            if rest.starts_with("/dev/null") {
                skip_section = true;
            } else if let Some(p) = rest.strip_prefix("b/") {
                if let Some(f) = current.as_mut() {
                    f.path = p.to_string();
                }
            }
            continue;
        }

        if line.starts_with("--- ") {
            continue;
        }

        if line.starts_with("Binary files ") {
            skip_section = true;
            continue;
        }

        if let Some(rest) = line.strip_prefix("@@ ") {
            if let Some(start) = parse_hunk_new_start(rest) {
                new_line = start;
                in_hunk = true;
            } else {
                in_hunk = false;
            }
            continue;
        }

        if !in_hunk {
            continue;
        }

        if line.starts_with("\\ ") {
            // "\ No newline at end of file" — skip, don't advance counters.
            continue;
        }

        if let Some(text) = line.strip_prefix('+') {
            if let Some(f) = current.as_mut() {
                f.added.push(AddedLine {
                    lineno: new_line,
                    text: text.to_string(),
                });
            }
            new_line += 1;
        } else if line.starts_with('-') {
            // Removed line — only consumes old-file counter.
        } else {
            // Context line (starts with space, or empty).
            new_line += 1;
        }
    }

    if let Some(f) = current {
        if !skip_section {
            out.push(f);
        }
    }
    out
}

/// Extract the `b/<path>` component from a `diff --git a/X b/Y` header tail.
fn extract_b_path(tail: &str) -> Option<String> {
    // Header format: `a/<path> b/<path>`; split at the space separating the
    // two quoted-or-bare paths. Git uses bare paths unless they contain odd
    // characters; we accept bare form only and fall back to the last `b/...`.
    let mut rest = tail;
    if let Some(space) = rest.find(" b/") {
        rest = &rest[space + 3..];
        return Some(rest.trim_end().to_string());
    }
    None
}

/// Parse the new-side start line from a hunk header body (after `@@ `).
///
/// Accepts both `-a,b +c,d @@...` and `-a +c @@...` shapes.
fn parse_hunk_new_start(body: &str) -> Option<u32> {
    // Find the `+` token.
    let plus_idx = body.find('+')?;
    let after = &body[plus_idx + 1..];
    // Read digits up to `,` or space.
    let end = after.find([',', ' ']).unwrap_or(after.len());
    after[..end].parse().ok()
}

/// Run all diff-only checks and return a sorted list of findings.
pub(crate) fn diff_checks(files: &[DiffFile], work_dir: &Path) -> Vec<String> {
    let mut findings: Vec<String> = Vec::new();
    for file in files {
        if !is_rust_file(&file.path) {
            continue;
        }
        findings.extend(check_unsafe_missing_safety(file, work_dir));
        findings.extend(check_hashmap_iteration(file));
        findings.extend(check_todo_fixme(file));
    }
    findings.sort();
    findings
}

fn is_rust_file(path: &str) -> bool {
    path.ends_with(".rs")
}

/// Flag added lines that open an `unsafe` block or fn without a `// SAFETY:`
/// comment within the 5 preceding lines of the on-disk file.
///
/// Reading from disk lets us see pre-existing SAFETY comments that weren't
/// touched by the diff, avoiding false positives when only the unsafe block
/// itself was added.
fn check_unsafe_missing_safety(file: &DiffFile, work_dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut file_lines: Option<Vec<String>> = None;
    for added in &file.added {
        if !line_opens_unsafe(&added.text) {
            continue;
        }
        // Lazily read the file once.
        if file_lines.is_none() {
            let abs = work_dir.join(&file.path);
            match std::fs::read_to_string(&abs) {
                Ok(s) => file_lines = Some(s.lines().map(|l| l.to_string()).collect()),
                Err(_) => {
                    file_lines = Some(Vec::new());
                }
            }
        }
        let lines = file_lines.as_ref().unwrap();
        let idx = added.lineno.saturating_sub(1) as usize;
        let start = idx.saturating_sub(5);
        let window = lines.get(start..idx).unwrap_or(&[]);
        let has_safety = window.iter().any(|l| l.contains("// SAFETY:"));
        if !has_safety {
            out.push(format!(
                "unsafe: {}:{}: new `unsafe` without `// SAFETY:` comment in the 5 preceding lines",
                file.path, added.lineno
            ));
        }
    }
    out
}

/// True if an added line opens an `unsafe` block or declares `unsafe fn`.
///
/// Skips comment lines and string contexts heuristically — we only match on
/// the leading non-whitespace token to avoid flagging `// unsafe {` notes.
fn line_opens_unsafe(text: &str) -> bool {
    let trimmed = text.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
        return false;
    }
    // `unsafe {`, `unsafe fn`, `pub unsafe fn`, `unsafe impl`.
    has_unsafe_keyword(trimmed)
}

fn has_unsafe_keyword(s: &str) -> bool {
    // Walk tokens separated by whitespace; look for `unsafe` followed by
    // `{`, `fn`, `impl`, or `trait`.
    let mut prev_is_unsafe = false;
    for tok in s.split_whitespace() {
        if prev_is_unsafe
            && (tok.starts_with('{') || tok == "fn" || tok == "impl" || tok == "trait")
        {
            return true;
        }
        // Trim trailing punctuation for token comparison.
        let clean = tok.trim_end_matches([',', ';']);
        prev_is_unsafe = clean == "unsafe";
    }
    false
}

/// Flag added lines that introduce `HashMap` or `HashSet` references, which
/// risk non-deterministic iteration order if they reach output paths.
fn check_hashmap_iteration(file: &DiffFile) -> Vec<String> {
    let mut out = Vec::new();
    for added in &file.added {
        if is_comment_line(&added.text) {
            continue;
        }
        if added.text.contains("HashMap") || added.text.contains("HashSet") {
            out.push(format!(
                "hash: {}:{}: new `HashMap`/`HashSet` mention — verify iteration order is sorted before reaching output",
                file.path, added.lineno
            ));
        }
    }
    out
}

/// Flag added lines that introduce a new `TODO` or `FIXME` marker.
fn check_todo_fixme(file: &DiffFile) -> Vec<String> {
    let mut out = Vec::new();
    for added in &file.added {
        let text = &added.text;
        if text.contains("TODO") || text.contains("FIXME") {
            out.push(format!(
                "todo: {}:{}: new `TODO`/`FIXME` marker",
                file.path, added.lineno
            ));
        }
    }
    out
}

fn is_comment_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

#[cfg(test)]
mod diff_only_tests {
    use super::*;

    fn line(n: u32, t: &str) -> AddedLine {
        AddedLine {
            lineno: n,
            text: t.to_string(),
        }
    }

    #[test]
    fn parses_simple_added_lines_with_correct_line_numbers() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1..2 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,5 @@
 one
 two
+three
+four
 five
";
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/lib.rs");
        assert_eq!(files[0].added, vec![line(3, "three"), line(4, "four")]);
    }

    #[test]
    fn parses_multiple_hunks() {
        let diff = "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,3 @@
 a
+b
 c
@@ -10,1 +11,2 @@
 x
+y
";
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].added, vec![line(2, "b"), line(12, "y")]);
    }

    #[test]
    fn skips_deleted_files() {
        let diff = "\
diff --git a/gone.rs b/gone.rs
--- a/gone.rs
+++ /dev/null
@@ -1,1 +0,0 @@
-bye
";
        let files = parse_diff(diff);
        assert!(files.is_empty());
    }

    #[test]
    fn handles_new_file_against_dev_null() {
        let diff = "\
diff --git a/new.rs b/new.rs
new file mode 100644
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,2 @@
+let x = 1;
+let y = 2;
";
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "new.rs");
        assert_eq!(
            files[0].added,
            vec![line(1, "let x = 1;"), line(2, "let y = 2;")]
        );
    }

    #[test]
    fn skips_binary_sections() {
        let diff = "\
diff --git a/img.png b/img.png
Binary files a/img.png and b/img.png differ
";
        let files = parse_diff(diff);
        assert!(files.is_empty());
    }

    #[test]
    fn check_todo_fixme_flags_added_markers() {
        let file = DiffFile {
            path: "a.rs".into(),
            added: vec![
                line(1, "// TODO: fix this"),
                line(2, "let x = 1;"),
                line(3, "// FIXME(T99): cleanup"),
            ],
        };
        let findings = check_todo_fixme(&file);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].contains("a.rs:1"));
        assert!(findings[1].contains("a.rs:3"));
    }

    #[test]
    fn check_hashmap_flags_new_mentions_but_ignores_comments() {
        let file = DiffFile {
            path: "a.rs".into(),
            added: vec![
                line(1, "use std::collections::HashMap;"),
                line(2, "// uses HashSet internally"),
                line(3, "let m: HashSet<u32> = HashSet::new();"),
            ],
        };
        let findings = check_hashmap_iteration(&file);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].contains("a.rs:1"));
        assert!(findings[1].contains("a.rs:3"));
    }

    #[test]
    fn has_unsafe_keyword_detects_common_shapes() {
        assert!(has_unsafe_keyword("unsafe { foo() }"));
        assert!(has_unsafe_keyword("pub unsafe fn bar()"));
        assert!(has_unsafe_keyword("unsafe impl Send for X {}"));
        assert!(has_unsafe_keyword("unsafe trait Marker {}"));
        assert!(!has_unsafe_keyword("let s = \"unsafe\";"));
        assert!(!has_unsafe_keyword("fn safe() { ok(); }"));
    }

    #[test]
    fn line_opens_unsafe_ignores_comment_lines() {
        assert!(!line_opens_unsafe("// unsafe { hack }"));
        assert!(!line_opens_unsafe(" * unsafe fn doc"));
        assert!(line_opens_unsafe("    unsafe { *p }"));
    }

    #[test]
    fn check_unsafe_flags_when_no_safety_comment_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = "a.rs";
        std::fs::write(
            dir.path().join(path),
            "fn caller() {\n    unsafe { ptr::read(p) }\n}\n",
        )
        .unwrap();
        let file = DiffFile {
            path: path.into(),
            added: vec![line(2, "    unsafe { ptr::read(p) }")],
        };
        let findings = check_unsafe_missing_safety(&file, dir.path());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("a.rs:2"));
    }

    #[test]
    fn check_unsafe_passes_when_safety_comment_in_preceding_context() {
        let dir = tempfile::tempdir().unwrap();
        let path = "a.rs";
        std::fs::write(
            dir.path().join(path),
            "fn caller() {\n    // SAFETY: p is non-null by construction.\n    unsafe { ptr::read(p) }\n}\n",
        )
        .unwrap();
        let file = DiffFile {
            path: path.into(),
            added: vec![line(3, "    unsafe { ptr::read(p) }")],
        };
        let findings = check_unsafe_missing_safety(&file, dir.path());
        assert!(findings.is_empty(), "unexpected: {findings:?}");
    }

    #[test]
    fn diff_checks_scopes_to_rust_files_only() {
        let files = vec![
            DiffFile {
                path: "README.md".into(),
                added: vec![line(1, "HashMap TODO unsafe { stuff }")],
            },
            DiffFile {
                path: "x.rs".into(),
                added: vec![line(1, "// TODO: real todo")],
            },
        ];
        let dir = tempfile::tempdir().unwrap();
        let findings = diff_checks(&files, dir.path());
        // Only the .rs file contributes.
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("x.rs:1"));
    }

    #[test]
    fn parse_hunk_new_start_accepts_both_shapes() {
        assert_eq!(parse_hunk_new_start("-1,3 +4,5 @@"), Some(4));
        assert_eq!(parse_hunk_new_start("-1 +4 @@"), Some(4));
        assert_eq!(parse_hunk_new_start("-0,0 +1,10 @@ fn x"), Some(1));
    }

    #[test]
    fn extract_b_path_reads_header_tail() {
        assert_eq!(
            extract_b_path("a/src/lib.rs b/src/lib.rs"),
            Some("src/lib.rs".into())
        );
        assert_eq!(
            extract_b_path("a/sub/foo.rs b/sub/foo.rs"),
            Some("sub/foo.rs".into())
        );
    }
}
