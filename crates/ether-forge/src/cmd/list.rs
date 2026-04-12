//! `ether-forge list` — tabular backlog listing sorted by priority then id.

use std::path::Path;

use anyhow::{bail, Result};

use crate::task::{Status, Task};

/// Render a listing of tasks in `backlog_dir`, optionally filtered by status.
pub fn run(backlog_dir: &Path, status_filter: Option<&str>) -> Result<()> {
    let filter = match status_filter {
        None => None,
        Some(s) => Some(parse_status(s)?),
    };
    let mut tasks = Task::load_all(backlog_dir)?;
    if let Some(s) = filter {
        tasks.retain(|t| t.status == s);
    }
    tasks.sort_by_key(|t| t.pick_key());
    let out = render(&tasks);
    print!("{out}");
    Ok(())
}

fn parse_status(s: &str) -> Result<Status> {
    Ok(match s {
        "draft" => Status::Draft,
        "ready" => Status::Ready,
        "blocked" => Status::Blocked,
        "done" => Status::Done,
        other => bail!("unknown status filter `{other}` (expected draft|ready|blocked|done)"),
    })
}

fn render(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return String::from("(no tasks)\n");
    }
    let id_w = tasks.iter().map(|t| t.id.len()).max().unwrap_or(2).max(2);
    let status_w = tasks
        .iter()
        .map(|t| t.status.as_str().len())
        .max()
        .unwrap_or(6)
        .max(6);
    let mut out = String::new();
    for t in tasks {
        let prio = t
            .priority
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());
        out.push_str(&format!(
            "{:<id_w$}  {:<status_w$}  {:>3}  {:<1}  {}\n",
            t.id,
            t.status.as_str(),
            prio,
            t.size.as_str(),
            t.title,
            id_w = id_w,
            status_w = status_w,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Size, Status};

    fn task(id: &str, priority: Option<u32>, status: Status) -> Task {
        Task {
            id: id.to_string(),
            title: format!("title-{id}"),
            size: Size::S,
            status,
            depends_on: vec![],
            priority,
            commit: None,
            body: String::new(),
        }
    }

    #[test]
    fn sorts_priority_then_id() {
        let mut tasks = [
            task("T10", Some(5), Status::Ready),
            task("T2", Some(1), Status::Ready),
            task("T3", None, Status::Ready),
            task("T4", Some(1), Status::Ready),
        ];
        tasks.sort_by_key(|t| t.pick_key());
        let ids: Vec<_> = tasks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids, vec!["T2", "T4", "T10", "T3"]);
    }

    #[test]
    fn render_empty_reports_no_tasks() {
        assert_eq!(render(&[]), "(no tasks)\n");
    }

    #[test]
    fn render_includes_every_task_id_and_title() {
        let tasks = vec![
            task("T2", Some(1), Status::Ready),
            task("T10", None, Status::Blocked),
        ];
        let out = render(&tasks);
        assert!(out.contains("T2"));
        assert!(out.contains("title-T2"));
        assert!(out.contains("T10"));
        assert!(out.contains("title-T10"));
        assert!(out.contains("ready"));
        assert!(out.contains("blocked"));
    }
}
