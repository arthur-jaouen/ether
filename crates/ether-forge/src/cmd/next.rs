//! `ether-forge next` — print the top `ready` task by priority then id.

use std::path::Path;

use anyhow::Result;

use crate::task::{Status, Task};

/// Print the next ready task (lowest priority, lowest id tiebreaker), or
/// `(none)` when no ready tasks exist.
pub fn run(backlog_dir: &Path) -> Result<()> {
    let tasks = Task::load_all(backlog_dir)?;
    match pick(&tasks) {
        Some(t) => println!("{}  {}", t.id, t.title),
        None => println!("(none)"),
    }
    Ok(())
}

/// Return the next-up task from a pre-loaded slice. Exposed for tests.
pub fn pick(tasks: &[Task]) -> Option<&Task> {
    tasks
        .iter()
        .filter(|t| t.status == Status::Ready)
        .min_by_key(|t| t.pick_key())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Size, Status};

    fn task(id: &str, priority: Option<u32>, status: Status) -> Task {
        Task {
            id: id.to_string(),
            title: format!("t{id}"),
            size: Size::S,
            status,
            depends_on: vec![],
            priority,
            commit: None,
            body: String::new(),
        }
    }

    #[test]
    fn picks_lowest_priority_ready() {
        let tasks = vec![
            task("T1", None, Status::Ready),
            task("T2", Some(5), Status::Ready),
            task("T3", Some(1), Status::Ready),
            task("T4", Some(1), Status::Blocked),
        ];
        assert_eq!(pick(&tasks).unwrap().id, "T3");
    }

    #[test]
    fn tiebreaks_by_numeric_id() {
        let tasks = vec![
            task("T10", Some(2), Status::Ready),
            task("T2", Some(2), Status::Ready),
        ];
        assert_eq!(pick(&tasks).unwrap().id, "T2");
    }

    #[test]
    fn returns_none_when_no_ready() {
        let tasks = vec![task("T1", None, Status::Blocked)];
        assert!(pick(&tasks).is_none());
    }

    #[test]
    fn unprioritized_loses_to_prioritized() {
        let tasks = vec![
            task("T1", None, Status::Ready),
            task("T99", Some(10), Status::Ready),
        ];
        assert_eq!(pick(&tasks).unwrap().id, "T99");
    }
}
