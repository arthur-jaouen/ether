//! `ether-forge rules-scan [T<n>]` — run every grep recipe against the diff.
//!
//! Auto-discovers recipes under `.claude/rules/grep/*.yml`, runs each one
//! against the added lines from `git diff main` (or a task-scoped diff when
//! an id is supplied), and emits a `{recipe: [{file, line, text}]}` JSON map
//! on stdout. Collapses the reviewer subagent's hand-rolled "read each rule
//! then grep" loop into a single call.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;

use crate::cmd::diff;
use crate::cmd::grep::{default_dir, list_recipes, Recipe};
use crate::cmd::validate::{parse_diff, AddedLine, DiffFile};

/// One match within a scanned diff — the new-side file path, the 1-based line
/// number in that file, and the full line text (without the leading `+`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Match {
    pub file: String,
    pub line: u32,
    pub text: String,
}

/// Run the rules-scan subcommand: discover recipes, diff main, match each
/// recipe against every added line, and emit JSON to stdout.
pub fn run(backlog_dir: &Path, task_id: Option<&str>) -> Result<()> {
    let work_dir = diff::resolve_work_dir(backlog_dir, task_id)?;
    let recipes = list_recipes(&default_dir())?;
    let raw_diff = diff::git_diff_main(&work_dir)?;
    let filtered = diff::filter_lockfiles(&raw_diff);
    let files = parse_diff(&filtered);

    let report = scan(&recipes, &files)?;
    let json = serde_json::to_string_pretty(&report).context("serializing rules-scan report")?;
    println!("{json}");
    Ok(())
}

/// Match every recipe against the added lines from `files` and group hits by
/// recipe name.
///
/// The output map always contains an entry for every recipe (empty vector on
/// no hits) so downstream consumers can tell "recipe ran clean" apart from
/// "recipe never loaded".
pub fn scan(recipes: &[Recipe], files: &[DiffFile]) -> Result<BTreeMap<String, Vec<Match>>> {
    let mut out: BTreeMap<String, Vec<Match>> = BTreeMap::new();
    for recipe in recipes {
        let re = Regex::new(&recipe.pattern)
            .with_context(|| format!("compiling recipe `{}` regex", recipe.name))?;
        let mut hits = match_recipe(&re, files, recipe.path.as_deref());
        // Sort for deterministic output — BTreeMap already sorts keys, but
        // per-recipe hit ordering needs to be explicit.
        hits.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
        out.insert(recipe.name.clone(), hits);
    }
    Ok(out)
}

/// Walk every added line in `files` and collect the ones matching `re`.
///
/// `path_filter` mirrors `Recipe::path` — when set, only files whose diff
/// path starts with that prefix contribute matches.
fn match_recipe(re: &Regex, files: &[DiffFile], path_filter: Option<&str>) -> Vec<Match> {
    let mut out = Vec::new();
    for file in files {
        if let Some(prefix) = path_filter {
            if !file.path.starts_with(prefix) {
                continue;
            }
        }
        for AddedLine { lineno, text } in &file.added {
            if re.is_match(text) {
                out.push(Match {
                    file: file.path.clone(),
                    line: *lineno,
                    text: text.clone(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(name: &str, pattern: &str, path: Option<&str>) -> Recipe {
        Recipe {
            name: name.into(),
            pattern: pattern.into(),
            path: path.map(str::to_string),
            description: None,
        }
    }

    fn file(path: &str, lines: &[(u32, &str)]) -> DiffFile {
        DiffFile {
            path: path.into(),
            added: lines
                .iter()
                .map(|(n, t)| AddedLine {
                    lineno: *n,
                    text: (*t).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn scan_reports_every_recipe_even_without_hits() {
        let recipes = vec![
            recipe("todo", r"\b(TODO|FIXME)\b", None),
            recipe("unsafe", r"\bunsafe\s*\{", None),
        ];
        let files = vec![file("a.rs", &[(1, "let x = 1;")])];
        let report = scan(&recipes, &files).unwrap();
        assert_eq!(report.len(), 2);
        assert!(report.get("todo").unwrap().is_empty());
        assert!(report.get("unsafe").unwrap().is_empty());
    }

    #[test]
    fn scan_collects_multiple_recipe_hits_per_file() {
        let recipes = vec![
            recipe("todo", r"\b(TODO|FIXME)\b", None),
            recipe("hash", r"\b(HashMap|HashSet)\b", None),
        ];
        let files = vec![file(
            "crates/x/src/lib.rs",
            &[
                (1, "// TODO: fix"),
                (2, "use std::collections::HashMap;"),
                (3, "let x: HashSet<u32> = HashSet::new();"),
                (4, "let z = 1; // FIXME later"),
            ],
        )];
        let report = scan(&recipes, &files).unwrap();

        let todos = report.get("todo").unwrap();
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].line, 1);
        assert_eq!(todos[1].line, 4);

        let hashes = report.get("hash").unwrap();
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes[0].line, 2);
        assert_eq!(hashes[1].line, 3);
    }

    #[test]
    fn scan_honors_recipe_path_filter() {
        let recipes = vec![recipe("todo", r"\bTODO\b", Some("crates"))];
        let files = vec![
            file("crates/x/src/lib.rs", &[(1, "// TODO: inside")]),
            file("docs/notes.md", &[(1, "// TODO: outside")]),
        ];
        let report = scan(&recipes, &files).unwrap();
        let hits = report.get("todo").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file, "crates/x/src/lib.rs");
    }

    #[test]
    fn scan_sorts_hits_by_file_then_line() {
        let recipes = vec![recipe("todo", r"TODO", None)];
        let files = vec![
            file("z.rs", &[(5, "TODO a"), (1, "TODO b")]),
            file("a.rs", &[(10, "TODO c")]),
        ];
        let report = scan(&recipes, &files).unwrap();
        let hits = report.get("todo").unwrap();
        // Expected sorted order: a.rs:10, z.rs:1, z.rs:5.
        assert_eq!(hits.len(), 3);
        assert_eq!((hits[0].file.as_str(), hits[0].line), ("a.rs", 10));
        assert_eq!((hits[1].file.as_str(), hits[1].line), ("z.rs", 1));
        assert_eq!((hits[2].file.as_str(), hits[2].line), ("z.rs", 5));
    }

    #[test]
    fn scan_errors_on_invalid_regex() {
        let recipes = vec![recipe("broken", "[unclosed", None)];
        let files: Vec<DiffFile> = Vec::new();
        let err = scan(&recipes, &files).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("broken"), "missing recipe name: {msg}");
    }

    #[test]
    fn scan_from_parsed_diff_end_to_end() {
        // Use the real parser so we catch wiring regressions between
        // parse_diff (validate.rs) and scan (rules_scan.rs).
        let diff = "\
diff --git a/crates/x/src/lib.rs b/crates/x/src/lib.rs
--- a/crates/x/src/lib.rs
+++ b/crates/x/src/lib.rs
@@ -1,2 +1,4 @@
 pub fn foo() {}
+// TODO: drop this
+use std::collections::HashMap;
 pub fn bar() {}
";
        let files = parse_diff(diff);
        let recipes = vec![
            recipe("todo", r"\bTODO\b", Some("crates")),
            recipe("hash", r"\bHashMap\b", Some("crates")),
        ];
        let report = scan(&recipes, &files).unwrap();
        assert_eq!(report.get("todo").unwrap().len(), 1);
        assert_eq!(report.get("hash").unwrap().len(), 1);
        assert_eq!(report.get("todo").unwrap()[0].line, 2);
        assert_eq!(report.get("hash").unwrap()[0].line, 3);
    }
}
