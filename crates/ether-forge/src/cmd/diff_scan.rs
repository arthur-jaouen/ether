//! Reviewer-subset source scans over a `git diff main` payload.
//!
//! Powers `ether-forge validate --diff-only`. The reviewer subagent used to
//! hand-roll these checks in prose — read the rules, look at the diff, grep
//! for `unsafe`/`HashMap`/`TODO` — which drifted and missed findings. This
//! module encodes the mechanical subset so the check runs the same way every
//! time.
//!
//! Scans operate on the *added* lines of a unified diff (the new side) so
//! findings are scoped to the review surface instead of re-flagging pre-
//! existing code. The parser recognizes enough of `git diff --unified`
//! output to pick file paths, hunk headers, and `+`/`-` content lines.
//!
//! The three checks:
//!
//! * **TODO/FIXME markers** — any added line containing `TODO`, `FIXME`,
//!   `XXX`, or `HACK` as a standalone word.
//! * **HashMap/HashSet** — any added line mentioning `HashMap` or `HashSet`
//!   as a standalone identifier, since iteration order is non-deterministic
//!   and the project requires sorted/BTreeMap equivalents (`CLAUDE.md` rules).
//! * **`unsafe` without `SAFETY:`** — any added line that opens an `unsafe`
//!   block/fn/impl, unless one of the three preceding added lines (same hunk)
//!   contains a `// SAFETY:` comment. The window is small on purpose: the
//!   convention is to place the comment immediately above the `unsafe`.

/// One file touched by the diff, with its added-line payload in cursor order.
///
/// `path` is the new-side path (`+++ b/<path>`). Deletion-only hunks are
/// still parsed so the file appears with an empty `added`, which keeps
/// downstream consumers from crashing on edge cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub added: Vec<AddedLine>,
}

/// A single added line plus its 1-based line number in the new file.
///
/// `content` does not include the leading `+` diff marker — it is the raw
/// source text as it would appear on disk after the patch is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedLine {
    pub lineno: u32,
    pub content: String,
}

/// Scan severity label attached to each finding so the caller can group
/// them under stable headings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Check {
    UnsafeWithoutSafety,
    NonDeterministicIter,
    TodoMarker,
}

impl Check {
    /// Short label for the category header in `validate` output.
    pub fn label(&self) -> &'static str {
        match self {
            Check::UnsafeWithoutSafety => "unsafe without SAFETY comment",
            Check::NonDeterministicIter => "non-deterministic HashMap/HashSet",
            Check::TodoMarker => "TODO/FIXME marker",
        }
    }
}

/// One finding emitted by a scan — carries the source location and a
/// short message. `lineno` is the new-side line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanFinding {
    pub check: Check,
    pub file: String,
    pub lineno: u32,
    pub message: String,
}

/// Parse a unified diff into per-file added-line lists.
///
/// Handles the `+++ b/<path>`, `@@ -... +new,count @@`, and `+`/`-`/` `
/// content lines emitted by `git diff main`. Deletions (`+++ /dev/null`)
/// are dropped. Binary diffs and rename-only hunks contribute empty
/// `added` lists but still appear in the output so callers can report
/// "file touched" without extra bookkeeping.
pub fn parse_diff(text: &str) -> Vec<FileDiff> {
    let mut out: Vec<FileDiff> = Vec::new();
    let mut current: Option<FileDiff> = None;
    let mut cursor: u32 = 0;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("+++ ") {
            if let Some(f) = current.take() {
                out.push(f);
            }
            // `+++ /dev/null` means the file was deleted on the new side.
            if rest == "/dev/null" {
                current = None;
            } else {
                let path = rest.strip_prefix("b/").unwrap_or(rest).to_string();
                current = Some(FileDiff {
                    path,
                    added: Vec::new(),
                });
            }
            cursor = 0;
            continue;
        }
        if line.starts_with("--- ")
            || line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("old mode")
            || line.starts_with("new mode")
            || line.starts_with("new file mode")
            || line.starts_with("deleted file mode")
            || line.starts_with("similarity index")
            || line.starts_with("rename from")
            || line.starts_with("rename to")
            || line.starts_with("copy from")
            || line.starts_with("copy to")
            || line.starts_with("Binary files")
            || line.starts_with("\\ No newline")
        {
            continue;
        }
        if let Some(hunk) = line.strip_prefix("@@ ") {
            // `@@ -old,count +new,count @@ optional context`
            if let Some(plus_tok) = hunk
                .split_whitespace()
                .find(|t| t.starts_with('+') && !t.starts_with("++"))
            {
                let digits: String = plus_tok[1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                cursor = digits.parse().unwrap_or(0);
            }
            continue;
        }
        let Some(file) = current.as_mut() else {
            continue;
        };
        if let Some(content) = line.strip_prefix('+') {
            file.added.push(AddedLine {
                lineno: cursor,
                content: content.to_string(),
            });
            cursor += 1;
        } else if line.starts_with('-') {
            // deletion — no cursor advance on the new side
        } else if line.starts_with(' ') || line.is_empty() {
            // context — advance new-side cursor
            cursor += 1;
        }
    }
    if let Some(f) = current {
        out.push(f);
    }
    out
}

/// Run all three scans against one file's added lines.
///
/// Findings are emitted in the order produced by each sub-scan (TODO →
/// HashMap → unsafe). The caller is responsible for final grouping and
/// deterministic output ordering.
pub fn scan_file(file: &FileDiff) -> Vec<ScanFinding> {
    let mut out = Vec::new();
    scan_todo(file, &mut out);
    scan_hashmap(file, &mut out);
    scan_unsafe(file, &mut out);
    out
}

fn scan_todo(file: &FileDiff, out: &mut Vec<ScanFinding>) {
    for line in &file.added {
        for marker in ["TODO", "FIXME", "XXX", "HACK"] {
            if contains_word(&line.content, marker) {
                out.push(ScanFinding {
                    check: Check::TodoMarker,
                    file: file.path.clone(),
                    lineno: line.lineno,
                    message: format!("new `{marker}` marker in added line"),
                });
                break;
            }
        }
    }
}

fn scan_hashmap(file: &FileDiff, out: &mut Vec<ScanFinding>) {
    for line in &file.added {
        let hit = ["HashMap", "HashSet"]
            .iter()
            .find(|ty| contains_word(&line.content, ty));
        if let Some(ty) = hit {
            out.push(ScanFinding {
                check: Check::NonDeterministicIter,
                file: file.path.clone(),
                lineno: line.lineno,
                message: format!(
                    "new `{ty}` reference — prefer `BTreeMap`/`BTreeSet` or sort before iterating"
                ),
            });
        }
    }
}

fn scan_unsafe(file: &FileDiff, out: &mut Vec<ScanFinding>) {
    for (idx, line) in file.added.iter().enumerate() {
        if !line_opens_unsafe(&line.content) {
            continue;
        }
        // Look at up to 3 preceding added lines in the same file for a
        // `SAFETY:` comment. 3 is deliberate: the convention is a single
        // rustdoc-style comment directly above the unsafe block.
        let start = idx.saturating_sub(3);
        let has_safety = file.added[start..idx]
            .iter()
            .any(|l| l.content.contains("SAFETY:"));
        if !has_safety {
            out.push(ScanFinding {
                check: Check::UnsafeWithoutSafety,
                file: file.path.clone(),
                lineno: line.lineno,
                message: "new `unsafe` without a `// SAFETY:` comment on the preceding lines"
                    .to_string(),
            });
        }
    }
}

/// True when `hay` contains `needle` as a standalone word.
///
/// Word boundaries are ASCII identifier characters (`[A-Za-z0-9_]`). Used
/// for `TODO`/`HashMap`/`unsafe` detection so `Todos` or `THashMap` don't
/// trip the scans.
fn contains_word(hay: &str, needle: &str) -> bool {
    let bytes = hay.as_bytes();
    let n = needle.len();
    let mut start = 0;
    while let Some(pos) = hay[start..].find(needle) {
        let abs = start + pos;
        let before_ok = abs == 0 || !is_ident_byte(bytes[abs - 1]);
        let end = abs + n;
        let after_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        start = abs + n;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// True when `line` opens an `unsafe` block, fn, or impl.
///
/// Matches the `\bunsafe\s*(fn|impl|\{)` pattern from
/// `.claude/rules/grep/unsafe-without-safety.yml` so the two stay in sync.
fn line_opens_unsafe(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut start = 0;
    while let Some(pos) = line[start..].find("unsafe") {
        let abs = start + pos;
        let before_ok = abs == 0 || !is_ident_byte(bytes[abs - 1]);
        let after_idx = abs + "unsafe".len();
        if before_ok {
            let tail = line[after_idx..].trim_start();
            if tail.starts_with('{') {
                return true;
            }
            if let Some(rest) = tail.strip_prefix("fn") {
                if rest.is_empty() || !is_ident_byte(rest.as_bytes()[0]) {
                    return true;
                }
            }
            if let Some(rest) = tail.strip_prefix("impl") {
                if rest.is_empty() || !is_ident_byte(rest.as_bytes()[0]) {
                    return true;
                }
            }
        }
        start = after_idx;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNSAFE_DIFF: &str = "\
diff --git a/crates/ether-core/src/storage.rs b/crates/ether-core/src/storage.rs
index 1111111..2222222 100644
--- a/crates/ether-core/src/storage.rs
+++ b/crates/ether-core/src/storage.rs
@@ -10,3 +10,6 @@ impl Storage {
 fn existing() {
     let x = 1;
 }
+unsafe fn raw_get(ptr: *const u8) -> u8 {
+    *ptr
+}
";

    const UNSAFE_WITH_SAFETY_DIFF: &str = "\
diff --git a/crates/ether-core/src/storage.rs b/crates/ether-core/src/storage.rs
index 1111111..2222222 100644
--- a/crates/ether-core/src/storage.rs
+++ b/crates/ether-core/src/storage.rs
@@ -10,3 +10,7 @@ impl Storage {
 fn existing() {
     let x = 1;
 }
+// SAFETY: caller guarantees ptr is non-null and aligned
+unsafe fn raw_get(ptr: *const u8) -> u8 {
+    *ptr
+}
";

    const HASHMAP_DIFF: &str = "\
diff --git a/crates/ether-core/src/world.rs b/crates/ether-core/src/world.rs
index aaa..bbb 100644
--- a/crates/ether-core/src/world.rs
+++ b/crates/ether-core/src/world.rs
@@ -5,2 +5,4 @@
 fn stable() {}
 fn other() {}
+use std::collections::HashMap;
+struct Store { inner: HashMap<u32, u32> }
";

    const TODO_DIFF: &str = "\
diff --git a/crates/ether-forge/src/main.rs b/crates/ether-forge/src/main.rs
index 123..456 100644
--- a/crates/ether-forge/src/main.rs
+++ b/crates/ether-forge/src/main.rs
@@ -20,1 +20,3 @@
 fn keep() {}
+// TODO: wire this up
+fn new_fn() {}
";

    #[test]
    fn parse_diff_extracts_path_and_added_lines() {
        let files = parse_diff(UNSAFE_DIFF);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "crates/ether-core/src/storage.rs");
        let contents: Vec<&str> = files[0].added.iter().map(|l| l.content.as_str()).collect();
        assert_eq!(
            contents,
            vec!["unsafe fn raw_get(ptr: *const u8) -> u8 {", "    *ptr", "}",]
        );
        // Hunk header was `+10,6`, three context lines come first, so the
        // first added line is at new-file line 13.
        assert_eq!(files[0].added[0].lineno, 13);
    }

    #[test]
    fn parse_diff_skips_deletions_and_devnull() {
        let diff = "\
diff --git a/old.txt b/old.txt
--- a/old.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-gone
-also gone
";
        let files = parse_diff(diff);
        assert!(
            files.is_empty(),
            "deleted-file hunk should not emit a FileDiff"
        );
    }

    #[test]
    fn parse_diff_handles_multiple_hunks() {
        let diff = "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,2 @@
 first
+added one
@@ -10,1 +11,2 @@
 tenth
+added two
";
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        let lines: Vec<u32> = files[0].added.iter().map(|l| l.lineno).collect();
        assert_eq!(lines, vec![2, 12]);
    }

    #[test]
    fn scan_unsafe_flags_new_unsafe_without_safety_comment() {
        let files = parse_diff(UNSAFE_DIFF);
        let findings = scan_file(&files[0]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check, Check::UnsafeWithoutSafety);
        assert_eq!(findings[0].file, "crates/ether-core/src/storage.rs");
        assert_eq!(findings[0].lineno, 13);
    }

    #[test]
    fn scan_unsafe_passes_when_safety_comment_is_in_added_lines_above() {
        let files = parse_diff(UNSAFE_WITH_SAFETY_DIFF);
        let findings = scan_file(&files[0]);
        assert!(
            findings
                .iter()
                .all(|f| f.check != Check::UnsafeWithoutSafety),
            "expected no unsafe finding, got {findings:?}"
        );
    }

    #[test]
    fn scan_hashmap_flags_added_hashmap_reference() {
        let files = parse_diff(HASHMAP_DIFF);
        let findings = scan_file(&files[0]);
        let hashmap: Vec<_> = findings
            .iter()
            .filter(|f| f.check == Check::NonDeterministicIter)
            .collect();
        assert_eq!(hashmap.len(), 2);
        assert!(hashmap.iter().all(|f| f.message.contains("HashMap")));
    }

    #[test]
    fn scan_hashmap_ignores_context_lines() {
        let diff = "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,2 +1,2 @@
 use std::collections::HashMap;
+let x = 1;
";
        let files = parse_diff(diff);
        let findings = scan_file(&files[0]);
        assert!(
            findings
                .iter()
                .all(|f| f.check != Check::NonDeterministicIter),
            "HashMap in context line must not be flagged, got {findings:?}"
        );
    }

    #[test]
    fn scan_todo_flags_added_marker() {
        let files = parse_diff(TODO_DIFF);
        let findings = scan_file(&files[0]);
        let todos: Vec<_> = findings
            .iter()
            .filter(|f| f.check == Check::TodoMarker)
            .collect();
        assert_eq!(todos.len(), 1);
        assert!(todos[0].message.contains("TODO"));
    }

    #[test]
    fn scan_todo_matches_all_four_markers() {
        for marker in ["TODO", "FIXME", "XXX", "HACK"] {
            let diff = format!(
                "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,2 @@
 stable
+// {marker}: do the thing
"
            );
            let files = parse_diff(&diff);
            let findings = scan_file(&files[0]);
            let todos: Vec<_> = findings
                .iter()
                .filter(|f| f.check == Check::TodoMarker)
                .collect();
            assert_eq!(
                todos.len(),
                1,
                "expected 1 hit for {marker}, got {findings:?}"
            );
            assert!(todos[0].message.contains(marker));
        }
    }

    #[test]
    fn scan_todo_does_not_match_substring() {
        let diff = "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,2 @@
 stable
+let _TODOS_ignored = 1;
";
        let files = parse_diff(diff);
        let findings = scan_file(&files[0]);
        // `_TODOS_` is an identifier, not the standalone word `TODO`.
        assert!(
            findings.iter().all(|f| f.check != Check::TodoMarker),
            "substring match must not fire, got {findings:?}"
        );
    }

    #[test]
    fn contains_word_respects_boundaries() {
        assert!(contains_word("// TODO: x", "TODO"));
        assert!(contains_word("TODO", "TODO"));
        assert!(contains_word("foo TODO bar", "TODO"));
        assert!(!contains_word("TODOS", "TODO"));
        assert!(!contains_word("xTODOx", "TODO"));
        assert!(!contains_word("foo_TODO", "TODO"));
    }

    #[test]
    fn line_opens_unsafe_matches_block_fn_and_impl() {
        assert!(line_opens_unsafe("    unsafe { do_it() }"));
        assert!(line_opens_unsafe("unsafe fn raw() {}"));
        assert!(line_opens_unsafe("unsafe impl Send for X {}"));
        // The scan is textual — `unsafe {` inside a line-comment still
        // fires. That is intentional: a `// SAFETY:` comment on the
        // preceding line is the way to silence it.
        assert!(line_opens_unsafe("// unsafe { note }"));
    }

    #[test]
    fn line_opens_unsafe_rejects_identifiers_containing_unsafe() {
        assert!(!line_opens_unsafe("let unsafely = 1;"));
        assert!(!line_opens_unsafe("fn unsafest() {}"));
        // Bare word with no following brace/fn/impl is not a block opener.
        assert!(!line_opens_unsafe("return unsafe_flag;"));
    }
}
