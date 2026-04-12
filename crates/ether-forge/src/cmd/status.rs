//! `ether-forge status` — compact summary of backlog state for SessionStart hooks.

use std::path::Path;

use anyhow::Result;

use crate::task::{Status, Task};

/// Print a short backlog summary: counts by status plus the next ready task.
pub fn run(backlog_dir: &Path) -> Result<()> {
    let tasks = Task::load_all(backlog_dir)?;
    print!("{}", render(&tasks));
    Ok(())
}

/// Build the rendered summary. Exposed for tests.
pub fn render(tasks: &[Task]) -> String {
    let mut counts = [0u32; 4];
    for t in tasks {
        let slot = match t.status {
            Status::Draft => 0,
            Status::Ready => 1,
            Status::Blocked => 2,
            Status::Done => 3,
        };
        counts[slot] += 1;
    }
    let total = tasks.len();
    let mut out = format!(
        "backlog: {total} tasks — {} ready, {} blocked, {} draft, {} done\n",
        counts[1], counts[2], counts[0], counts[3],
    );
    match super::next::pick(tasks) {
        Some(t) => out.push_str(&format!("next: {}  {}\n", t.id, t.title)),
        None => out.push_str("next: (none)\n"),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Size, Status};

    fn task(id: &str, status: Status, priority: Option<u32>) -> Task {
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
    fn counts_and_next() {
        let tasks = vec![
            task("T1", Status::Ready, Some(1)),
            task("T2", Status::Ready, Some(2)),
            task("T3", Status::Blocked, None),
            task("T4", Status::Draft, None),
        ];
        let out = render(&tasks);
        assert!(out.contains("4 tasks"));
        assert!(out.contains("2 ready"));
        assert!(out.contains("1 blocked"));
        assert!(out.contains("1 draft"));
        assert!(out.contains("0 done"));
        assert!(out.contains("next: T1"));
    }

    #[test]
    fn no_ready_reports_none() {
        let tasks = vec![task("T1", Status::Blocked, None)];
        let out = render(&tasks);
        assert!(out.contains("next: (none)"));
    }

    #[test]
    fn empty_backlog() {
        let out = render(&[]);
        assert!(out.contains("0 tasks"));
        assert!(out.contains("next: (none)"));
    }
}
