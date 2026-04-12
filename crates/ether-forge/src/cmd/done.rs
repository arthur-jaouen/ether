//! `ether-forge done` — mark a task complete and cascade dependency updates.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use crate::frontmatter::Frontmatter;

/// Mark `id` as done inside `backlog_dir`, optionally recording a commit sha,
/// then cascade the completion across every remaining task's `depends_on`.
///
/// `backlog_dir` is the active backlog directory (e.g. `backlog/`). The done
/// archive is assumed to be `<backlog_dir>/done/`.
pub fn run(backlog_dir: &Path, id: &str, commit: Option<&str>) -> Result<()> {
    let done_dir = backlog_dir.join("done");
    if !done_dir.exists() {
        fs::create_dir_all(&done_dir)
            .with_context(|| format!("creating done dir {}", done_dir.display()))?;
    }

    let target = find_task_file(backlog_dir, id)?;
    complete_target(&target, &done_dir, id, commit)?;
    cascade(backlog_dir, id)?;
    Ok(())
}

fn find_task_file(dir: &Path, id: &str) -> Result<PathBuf> {
    let prefix = format!("{id}-");
    let entries =
        fs::read_dir(dir).with_context(|| format!("reading backlog dir {}", dir.display()))?;
    let mut matches: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("md")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.starts_with(&prefix) || n == format!("{id}.md"))
                    .unwrap_or(false)
        })
        .collect();
    matches.sort();
    match matches.len() {
        0 => Err(anyhow!("no active task file for {id} in {}", dir.display())),
        1 => Ok(matches.pop().unwrap()),
        _ => Err(anyhow!(
            "multiple task files match {id}: {matches:?} — resolve manually"
        )),
    }
}

fn complete_target(path: &Path, done_dir: &Path, id: &str, commit: Option<&str>) -> Result<()> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading task file {}", path.display()))?;
    let (fm_text, body) = split_frontmatter(&raw)
        .with_context(|| format!("parsing frontmatter in {}", path.display()))?;
    let mut fm = Frontmatter::parse(fm_text)?;

    match fm.scalar("status") {
        Some("done") => bail!("{id} is already done"),
        Some("blocked") => {
            let remaining = fm.list_items("depends_on");
            if !remaining.is_empty() {
                bail!("{id} has unsatisfied depends_on: {remaining:?}");
            }
        }
        _ => {}
    }
    if !fm.list_items("depends_on").is_empty() {
        bail!(
            "{id} has unsatisfied depends_on: {:?}",
            fm.list_items("depends_on")
        );
    }

    fm.remove("depends_on");
    fm.set_scalar("status", "done");
    if let Some(sha) = commit {
        fm.set_scalar("commit", sha);
    }

    let new_body = strip_sub_steps(body);
    let new_raw = render(&fm, &new_body);

    let dest = done_dir.join(
        path.file_name()
            .ok_or_else(|| anyhow!("task path has no file name"))?,
    );
    if dest.exists() {
        bail!("destination already exists: {}", dest.display());
    }
    fs::write(&dest, new_raw).with_context(|| format!("writing done file {}", dest.display()))?;
    fs::remove_file(path)
        .with_context(|| format!("removing active task file {}", path.display()))?;
    Ok(())
}

fn cascade(backlog_dir: &Path, completed: &str) -> Result<()> {
    let entries = fs::read_dir(backlog_dir)
        .with_context(|| format!("reading backlog dir {}", backlog_dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("md")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.starts_with('T'))
                    .unwrap_or(false)
        })
        .collect();
    files.sort();

    for path in files {
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let (fm_text, body) = split_frontmatter(&raw)
            .with_context(|| format!("parsing frontmatter in {}", path.display()))?;
        let mut fm = Frontmatter::parse(fm_text)?;

        if !fm.list_items("depends_on").contains(&completed.to_string()) {
            continue;
        }
        let now_empty = fm.remove_list_item("depends_on", completed)?;
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

/// Remove the `## Sub-steps` section (up to the next top-level heading or EOF).
fn strip_sub_steps(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("## Sub-steps") {
            i += 1;
            while i < lines.len() && !lines[i].starts_with("## ") {
                i += 1;
            }
            continue;
        }
        out.push_str(lines[i]);
        out.push('\n');
        i += 1;
    }
    // Trim trailing blank lines but keep a single trailing newline if non-empty.
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
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

fn split_frontmatter(raw: &str) -> Result<(&str, &str)> {
    let rest = raw
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("missing opening `---` fence"))?;
    let end = rest
        .find("\n---\n")
        .or_else(|| rest.find("\n---"))
        .ok_or_else(|| anyhow!("missing closing `---` fence"))?;
    let frontmatter = &rest[..end];
    let after = &rest[end + 1..];
    let body = after
        .strip_prefix("---\n")
        .unwrap_or(after)
        .trim_start_matches('\n');
    Ok((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_sub_steps_removes_section() {
        let body = "## Sub-steps\n\n- [ ] one\n- [ ] two\n";
        assert_eq!(strip_sub_steps(body), "");
    }

    #[test]
    fn strip_sub_steps_keeps_other_sections() {
        let body = "## Notes\n\nprose\n\n## Sub-steps\n\n- [ ] one\n";
        let out = strip_sub_steps(body);
        assert!(out.contains("## Notes"));
        assert!(out.contains("prose"));
        assert!(!out.contains("Sub-steps"));
    }
}
