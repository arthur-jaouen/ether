//! `ether-forge get T<n>` — print full task file contents (frontmatter + body).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

/// Print the raw contents of the backlog task file matching `id`. Searches the
/// active backlog dir first, then the `done/` archive.
pub fn run(backlog_dir: &Path, id: &str) -> Result<()> {
    let path = locate(backlog_dir, id)?;
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading task file {}", path.display()))?;
    print!("{raw}");
    if !raw.ends_with('\n') {
        println!();
    }
    Ok(())
}

/// Locate the task file for `id` by searching active then done directories.
pub fn locate(backlog_dir: &Path, id: &str) -> Result<PathBuf> {
    if let Some(p) = find_in(backlog_dir, id)? {
        return Ok(p);
    }
    let done = backlog_dir.join("done");
    if done.exists() {
        if let Some(p) = find_in(&done, id)? {
            return Ok(p);
        }
    }
    Err(anyhow!("no task file found for {id}"))
}

fn find_in(dir: &Path, id: &str) -> Result<Option<PathBuf>> {
    let prefix = format!("{id}-");
    let exact = format!("{id}.md");
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading dir {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("md")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.starts_with(&prefix) || n == exact)
                    .unwrap_or(false)
        })
        .collect();
    matches.sort();
    match matches.len() {
        0 => Ok(None),
        1 => Ok(Some(matches.pop().unwrap())),
        _ => Err(anyhow!(
            "multiple task files match {id}: {matches:?} — resolve manually"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_finds_active_task() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("T7-demo.md"), "---\nid: T7\n---\n").unwrap();
        let p = locate(dir.path(), "T7").unwrap();
        assert!(p.ends_with("T7-demo.md"));
    }

    #[test]
    fn locate_finds_done_task() {
        let dir = tempfile::tempdir().unwrap();
        let done = dir.path().join("done");
        fs::create_dir(&done).unwrap();
        fs::write(done.join("T3-old.md"), "---\nid: T3\n---\n").unwrap();
        let p = locate(dir.path(), "T3").unwrap();
        assert!(p.ends_with("T3-old.md"));
    }

    #[test]
    fn locate_errors_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(locate(dir.path(), "T42").is_err());
    }

    #[test]
    fn locate_errors_on_ambiguous_match() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("T5-a.md"), "x").unwrap();
        fs::write(dir.path().join("T5-b.md"), "x").unwrap();
        assert!(locate(dir.path(), "T5").is_err());
    }
}
