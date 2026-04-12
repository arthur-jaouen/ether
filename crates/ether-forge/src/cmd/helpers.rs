//! `ether-forge helpers` — scan `crates/*/tests/common/mod.rs` and emit every
//! shared test helper with its owning crate.
//!
//! The review subagent uses this registry to spot duplicated test fixtures
//! across crates (the anti-pattern called out in `CLAUDE.md`). Output is
//! sorted by `(helper_name, crate)` so duplicate names cluster on adjacent
//! lines and a `[DUPLICATE]` marker makes them grep-friendly.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// One helper function discovered in a `tests/common/mod.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Helper {
    pub name: String,
    pub crate_name: String,
    pub path: PathBuf,
}

/// Walk `crates_dir` for every `<crate>/tests/common/mod.rs` and extract its
/// function definitions. Missing `crates_dir` returns an empty list so the
/// subcommand is safe to run from any working directory.
pub fn scan(crates_dir: &Path) -> Result<Vec<Helper>> {
    if !crates_dir.exists() {
        return Ok(Vec::new());
    }
    let mut crate_entries: Vec<PathBuf> = fs::read_dir(crates_dir)
        .with_context(|| format!("reading crates dir {}", crates_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    crate_entries.sort();

    let mut out = Vec::new();
    for crate_path in crate_entries {
        let common = crate_path.join("tests").join("common").join("mod.rs");
        if !common.exists() {
            continue;
        }
        let crate_name = crate_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let src =
            fs::read_to_string(&common).with_context(|| format!("reading {}", common.display()))?;
        for name in extract_fn_names(&src) {
            out.push(Helper {
                name,
                crate_name: crate_name.clone(),
                path: common.clone(),
            });
        }
    }
    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.crate_name.cmp(&b.crate_name))
    });
    Ok(out)
}

/// Pull function identifiers out of a Rust source string. Only matches
/// top-level `fn`/`pub fn`/`pub(...) fn` definitions — good enough for the
/// flat layout of a `tests/common/mod.rs` helper file without dragging in a
/// full parser.
pub fn extract_fn_names(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    for raw in src.lines() {
        let mut rest = raw.trim_start();
        if let Some(after_pub) = rest.strip_prefix("pub") {
            // Optional restriction like `pub(crate)` / `pub(super)`.
            let after_restriction = if let Some(open) = after_pub.strip_prefix('(') {
                match open.find(')') {
                    Some(i) => &open[i + 1..],
                    None => continue,
                }
            } else {
                after_pub
            };
            rest = after_restriction.trim_start();
        }
        let Some(after_fn) = rest.strip_prefix("fn ") else {
            continue;
        };
        let ident: String = after_fn
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !ident.is_empty() {
            names.push(ident);
        }
    }
    names
}

/// Render the scan result as human text. Duplicate names (appearing in more
/// than one crate) are tagged with `[DUPLICATE]` so a reviewer can eyeball
/// collisions at a glance.
pub fn render(helpers: &[Helper]) -> String {
    if helpers.is_empty() {
        return "No shared test helpers found under crates/*/tests/common/mod.rs.\n".to_string();
    }
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for h in helpers {
        *counts.entry(h.name.as_str()).or_insert(0) += 1;
    }
    let mut out = String::new();
    for h in helpers {
        let dup = if counts.get(h.name.as_str()).copied().unwrap_or(0) > 1 {
            " [DUPLICATE]"
        } else {
            ""
        };
        out.push_str(&format!("{}\t{}{}\n", h.crate_name, h.name, dup));
    }
    out
}

/// Run `ether-forge helpers`. Scans `./crates` by default.
pub fn run(crates_dir: &Path) -> Result<()> {
    let helpers = scan(crates_dir)?;
    print!("{}", render(&helpers));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_common(root: &Path, crate_name: &str, body: &str) {
        let dir = root.join(crate_name).join("tests").join("common");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("mod.rs"), body).unwrap();
    }

    #[test]
    fn extract_fn_names_handles_visibility_and_generics() {
        let src = r#"
pub fn spawn_test_world() -> World { World::new() }
fn private_helper() {}
pub(crate) fn crate_visible() {}
pub fn generic_one<T: Component>(t: T) {}
// fn commented_out() {}
"#;
        let names = extract_fn_names(src);
        assert_eq!(
            names,
            vec![
                "spawn_test_world",
                "private_helper",
                "crate_visible",
                "generic_one",
            ]
        );
    }

    #[test]
    fn scan_missing_crates_dir_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let helpers = scan(&tmp.path().join("nope")).unwrap();
        assert!(helpers.is_empty());
    }

    #[test]
    fn scan_sorted_by_name_then_crate_with_duplicates_highlighted() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_common(
            root,
            "ether-core",
            "pub fn spawn_test_world() {}\npub fn unique_core() {}\n",
        );
        write_common(
            root,
            "ether-macros",
            "pub fn spawn_test_world() {}\npub fn unique_macros() {}\n",
        );

        let helpers = scan(root).unwrap();
        let ordered: Vec<(&str, &str)> = helpers
            .iter()
            .map(|h| (h.name.as_str(), h.crate_name.as_str()))
            .collect();
        assert_eq!(
            ordered,
            vec![
                ("spawn_test_world", "ether-core"),
                ("spawn_test_world", "ether-macros"),
                ("unique_core", "ether-core"),
                ("unique_macros", "ether-macros"),
            ]
        );

        let out = render(&helpers);
        let dup_lines: Vec<&str> = out.lines().filter(|l| l.contains("[DUPLICATE]")).collect();
        assert_eq!(dup_lines.len(), 2);
        assert!(dup_lines.iter().all(|l| l.contains("spawn_test_world")));
        assert!(!out.contains("unique_core\t") || !out.contains("unique_core [DUPLICATE]"));
        assert!(!out.contains("unique_macros [DUPLICATE]"));
    }

    #[test]
    fn scan_skips_crates_without_common_mod() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("ether-core").join("src")).unwrap();
        write_common(root, "ether-macros", "pub fn only_helper() {}\n");

        let helpers = scan(root).unwrap();
        assert_eq!(helpers.len(), 1);
        assert_eq!(helpers[0].name, "only_helper");
        assert_eq!(helpers[0].crate_name, "ether-macros");
    }

    #[test]
    fn render_reports_empty_registry() {
        let out = render(&[]);
        assert!(out.contains("No shared test helpers"));
    }
}
