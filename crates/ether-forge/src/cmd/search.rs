//! `ether-forge search <query>` — case-insensitive substring match on id, title, and body.

use std::path::Path;

use anyhow::Result;

use crate::task::Task;

/// Print every task whose id, title, or body matches `query` (case-insensitive).
pub fn run(backlog_dir: &Path, query: &str) -> Result<()> {
    let mut tasks = Task::load_all(backlog_dir)?;
    tasks.sort_by_key(|t| t.numeric_id());
    let hits = filter(&tasks, query);
    let out = render(&hits);
    print!("{out}");
    Ok(())
}

/// Return every task whose id/title/body contains `query`, case-insensitively.
pub fn filter<'a>(tasks: &'a [Task], query: &str) -> Vec<&'a Task> {
    let needle = query.to_lowercase();
    tasks
        .iter()
        .filter(|t| {
            t.id.to_lowercase().contains(&needle)
                || t.title.to_lowercase().contains(&needle)
                || t.body.to_lowercase().contains(&needle)
        })
        .collect()
}

fn render(tasks: &[&Task]) -> String {
    if tasks.is_empty() {
        return String::from("(no matches)\n");
    }
    let mut out = String::new();
    for t in tasks {
        out.push_str(&format!("{}  {}\n", t.id, t.title));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Size, Status};

    fn task(id: &str, title: &str, body: &str) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            size: Size::S,
            status: Status::Ready,
            depends_on: vec![],
            priority: None,
            commit: None,
            body: body.to_string(),
        }
    }

    #[test]
    fn matches_title_case_insensitively() {
        let tasks = vec![task("T1", "Rewrite Query engine", "")];
        let hits = filter(&tasks, "query");
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn matches_body_substep() {
        let tasks = vec![
            task("T1", "x", "- [ ] implement archetype store"),
            task("T2", "y", "- [ ] sparse set"),
        ];
        let hits = filter(&tasks, "ARCHETYPE");
        let ids: Vec<_> = hits.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["T1"]);
    }

    #[test]
    fn matches_id_token() {
        let tasks = vec![task("T42", "Foo", ""), task("T7", "Bar", "")];
        let hits = filter(&tasks, "t42");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "T42");
    }

    #[test]
    fn no_matches_reports_empty() {
        let tasks = vec![task("T1", "Foo", "body")];
        let hits = filter(&tasks, "zzz");
        assert!(hits.is_empty());
        assert_eq!(render(&hits), "(no matches)\n");
    }
}
