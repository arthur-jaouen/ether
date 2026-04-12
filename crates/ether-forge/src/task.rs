use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

/// Task size classification. Mirrors the backlog schema in `BACKLOG.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Size {
    S,
    M,
    L,
}

/// Task lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Ready,
    Blocked,
    Done,
}

/// A backlog task parsed from a `backlog/T<n>-*.md` file.
#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub size: Size,
    pub status: Status,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub priority: Option<u32>,
    #[serde(default)]
    pub commit: Option<String>,
    /// Markdown body after the frontmatter (no leading newline).
    #[serde(skip)]
    pub body: String,
}

impl Status {
    /// Lowercase label matching the YAML frontmatter representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Ready => "ready",
            Status::Blocked => "blocked",
            Status::Done => "done",
        }
    }
}

impl Size {
    /// Uppercase label matching the YAML frontmatter representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Size::S => "S",
            Size::M => "M",
            Size::L => "L",
        }
    }
}

impl Task {
    /// Numeric portion of the task id (e.g. `T42` → `42`). Returns `u32::MAX`
    /// when the id is malformed so tasks with bad ids sort last.
    pub fn numeric_id(&self) -> u32 {
        parse_id_num(&self.id).unwrap_or(u32::MAX)
    }

    /// Ordering key used by `list` and `next`: priority ascending (missing =
    /// last), then numeric id ascending.
    pub fn pick_key(&self) -> (u32, u32) {
        (self.priority.unwrap_or(u32::MAX), self.numeric_id())
    }

    /// Load a single task file, splitting YAML frontmatter from markdown body.
    pub fn load(path: &Path) -> Result<Task> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading task file {}", path.display()))?;
        let (frontmatter, body) = split_frontmatter(&raw)
            .with_context(|| format!("parsing frontmatter in {}", path.display()))?;
        let mut task: Task = serde_yaml::from_str(frontmatter)
            .with_context(|| format!("deserializing frontmatter in {}", path.display()))?;
        task.body = body.to_string();
        Ok(task)
    }

    /// Load every `T*.md` task from `dir`, sorted deterministically by numeric ID.
    pub fn load_all(dir: &Path) -> Result<Vec<Task>> {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .with_context(|| format!("reading backlog dir {}", dir.display()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some("md")
                    && p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|n| n.starts_with('T'))
                        .unwrap_or(false)
            })
            .collect();
        entries.sort();

        let mut tasks: Vec<Task> = entries
            .iter()
            .map(|p| Task::load(p))
            .collect::<Result<_>>()?;
        tasks.sort_by_key(|t| parse_id_num(&t.id).unwrap_or(u32::MAX));
        Ok(tasks)
    }
}

fn split_frontmatter(raw: &str) -> Result<(&str, &str)> {
    let rest = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"))
        .ok_or_else(|| anyhow!("missing opening `---` fence"))?;
    let end = rest
        .find("\n---\n")
        .or_else(|| rest.find("\n---\r\n"))
        .ok_or_else(|| anyhow!("missing closing `---` fence"))?;
    let frontmatter = &rest[..end];
    let after = &rest[end + 1..];
    let body = after
        .strip_prefix("---\n")
        .or_else(|| after.strip_prefix("---\r\n"))
        .unwrap_or(after)
        .trim_start_matches('\n');
    Ok((frontmatter, body))
}

fn parse_id_num(id: &str) -> Option<u32> {
    id.strip_prefix('T').and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut f = fs::File::create(dir.path().join(name)).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn parses_valid_frontmatter() {
        let raw = "---\nid: T42\ntitle: Demo task\nsize: M\nstatus: ready\npriority: 3\n---\n\nBody text here.\n";
        let dir = write_tmp("T42-demo.md", raw);
        let task = Task::load(&dir.path().join("T42-demo.md")).unwrap();
        assert_eq!(task.id, "T42");
        assert_eq!(task.title, "Demo task");
        assert_eq!(task.size, Size::M);
        assert_eq!(task.status, Status::Ready);
        assert_eq!(task.priority, Some(3));
        assert!(task.depends_on.is_empty());
        assert!(task.commit.is_none());
        assert_eq!(task.body.trim(), "Body text here.");
    }

    #[test]
    fn parses_blocked_with_dependencies() {
        let raw = "---\nid: T7\ntitle: Blocked one\nsize: S\nstatus: blocked\ndepends_on:\n  - T5\n  - T6\n---\n\n";
        let dir = write_tmp("T7-blocked.md", raw);
        let task = Task::load(&dir.path().join("T7-blocked.md")).unwrap();
        assert_eq!(task.status, Status::Blocked);
        assert_eq!(task.depends_on, vec!["T5".to_string(), "T6".to_string()]);
    }

    #[test]
    fn rejects_missing_fence() {
        let raw = "id: T1\ntitle: no fence\n";
        let dir = write_tmp("T1-bad.md", raw);
        let err = Task::load(&dir.path().join("T1-bad.md")).unwrap_err();
        assert!(format!("{err:#}").contains("---"));
    }

    #[test]
    fn rejects_malformed_yaml() {
        let raw = "---\nid: T1\ntitle: [unterminated\n---\n\n";
        let dir = write_tmp("T1-bad.md", raw);
        assert!(Task::load(&dir.path().join("T1-bad.md")).is_err());
    }

    #[test]
    fn load_all_sorts_by_numeric_id() {
        let dir = tempfile::tempdir().unwrap();
        let files = [("T10-a.md", "T10"), ("T2-b.md", "T2"), ("T1-c.md", "T1")];
        for (name, id) in files {
            let raw = format!("---\nid: {id}\ntitle: t\nsize: S\nstatus: ready\n---\n\n");
            fs::write(dir.path().join(name), raw).unwrap();
        }
        let tasks = Task::load_all(dir.path()).unwrap();
        let ids: Vec<_> = tasks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids, vec!["T1", "T2", "T10"]);
    }

    #[test]
    fn round_trip_real_task_file() {
        let raw = "---\nid: T5\ntitle: Scaffold ether-forge crate and frontmatter parser\nsize: M\nstatus: ready\npriority: 1\n---\n\n## Sub-steps\n\n- [ ] step one\n";
        let dir = write_tmp("T5-ether-forge-scaffold.md", raw);
        let task = Task::load(&dir.path().join("T5-ether-forge-scaffold.md")).unwrap();
        assert_eq!(task.id, "T5");
        assert_eq!(task.size, Size::M);
        assert_eq!(task.priority, Some(1));
        assert!(task.body.contains("Sub-steps"));
    }
}
