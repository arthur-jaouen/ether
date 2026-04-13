//! `ether-forge rules` — concatenate or list CLAUDE.md and `.claude/rules/**/*.md`.
//!
//! Replaces the reviewer subagent's step 1 (reading CLAUDE.md and every rule
//! file individually) with a single forge call.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Run `ether-forge rules cat`: print CLAUDE.md, then every rule file, with separators.
pub fn cat(repo_root: &Path) -> Result<()> {
    print!("{}", render_cat(repo_root)?);
    Ok(())
}

/// Run `ether-forge rules list`: print the resolved file paths, one per line.
pub fn list(repo_root: &Path) -> Result<()> {
    for path in resolve(repo_root)? {
        println!("{}", rel_display(repo_root, &path));
    }
    Ok(())
}

/// Build the concatenated blob. Exposed for tests.
pub fn render_cat(repo_root: &Path) -> Result<String> {
    let mut out = String::new();
    for path in resolve(repo_root)? {
        let body =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        out.push_str(&format!("# --- {} ---\n", rel_display(repo_root, &path)));
        out.push_str(&body);
        if !body.ends_with('\n') {
            out.push('\n');
        }
    }
    Ok(out)
}

/// Resolve the ordered list of files: `CLAUDE.md` first, then every
/// `.claude/rules/**/*.md` sorted lexicographically. Missing `CLAUDE.md` or a
/// missing rules directory are both non-errors — they just contribute nothing.
fn resolve(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let claude_md = repo_root.join("CLAUDE.md");
    if claude_md.is_file() {
        out.push(claude_md);
    }
    let rules_dir = repo_root.join(".claude").join("rules");
    if rules_dir.is_dir() {
        let mut rule_files = Vec::new();
        collect_md(&rules_dir, &mut rule_files)?;
        rule_files.sort();
        out.extend(rule_files);
    }
    Ok(out)
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("stat {}", path.display()))?;
        if file_type.is_dir() {
            collect_md(&path, out)?;
        } else if file_type.is_file() && path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
    Ok(())
}

fn rel_display(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn cat_orders_claude_then_sorted_rules() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(&root.join("CLAUDE.md"), "top\n");
        write(&root.join(".claude/rules/b.md"), "beta\n");
        write(&root.join(".claude/rules/a.md"), "alpha\n");
        write(&root.join(".claude/rules/nested/c.md"), "gamma\n");
        // non-md file should be ignored
        write(&root.join(".claude/rules/ignore.txt"), "nope\n");

        let out = render_cat(root).unwrap();
        let claude_idx = out.find("# --- CLAUDE.md ---").unwrap();
        let a_idx = out.find("a.md").unwrap();
        let b_idx = out.find("b.md").unwrap();
        let c_idx = out.find("nested/c.md").unwrap();
        assert!(claude_idx < a_idx);
        assert!(a_idx < b_idx);
        assert!(b_idx < c_idx);
        assert!(out.contains("alpha\n"));
        assert!(out.contains("beta\n"));
        assert!(out.contains("gamma\n"));
        assert!(!out.contains("nope"));
    }

    #[test]
    fn cat_without_trailing_newline_is_normalized() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(&root.join("CLAUDE.md"), "no-newline");
        let out = render_cat(root).unwrap();
        assert!(out.contains("# --- CLAUDE.md ---\nno-newline\n"));
    }

    #[test]
    fn missing_claude_and_rules_is_empty() {
        let dir = tempdir().unwrap();
        let out = render_cat(dir.path()).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn missing_rules_dir_still_emits_claude() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(&root.join("CLAUDE.md"), "top\n");
        let out = render_cat(root).unwrap();
        assert!(out.starts_with("# --- CLAUDE.md ---\n"));
        assert!(out.contains("top\n"));
    }

    #[test]
    fn resolve_lists_expected_paths() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(&root.join("CLAUDE.md"), "x\n");
        write(&root.join(".claude/rules/a.md"), "x\n");
        write(&root.join(".claude/rules/zz/b.md"), "x\n");
        let paths: Vec<String> = resolve(root)
            .unwrap()
            .iter()
            .map(|p| rel_display(root, p))
            .collect();
        assert_eq!(
            paths,
            vec!["CLAUDE.md", ".claude/rules/a.md", ".claude/rules/zz/b.md"]
        );
    }
}
