//! `ether-forge deps T<n>` — print upward dependency tree and downward dependents.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{anyhow, Result};

use crate::task::Task;

/// Print the dependency tree rooted at `id`: upward (what it depends on,
/// transitively) and downward (what transitively depends on it).
pub fn run(backlog_dir: &Path, id: &str) -> Result<()> {
    let tasks = Task::load_all(backlog_dir)?;
    if !tasks.iter().any(|t| t.id == id) {
        return Err(anyhow!("task {id} not found in {}", backlog_dir.display()));
    }
    let out = render(&tasks, id);
    print!("{out}");
    Ok(())
}

/// Build a deterministic textual report of upward and downward dependencies
/// for `id` given the full task set.
pub fn render(tasks: &[Task], id: &str) -> String {
    let by_id: BTreeMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut out = String::new();
    out.push_str(&format!("{id}\n"));

    out.push_str("  depends on:\n");
    let mut up_seen = BTreeSet::new();
    walk_up(id, &by_id, &mut up_seen, 2, &mut out);
    if up_seen.is_empty() {
        out.push_str("    (none)\n");
    }

    out.push_str("  dependents:\n");
    let dependents = build_reverse(tasks);
    let mut down_seen = BTreeSet::new();
    walk_down(id, &dependents, &by_id, &mut down_seen, 2, &mut out);
    if down_seen.is_empty() {
        out.push_str("    (none)\n");
    }
    out
}

fn walk_up(
    id: &str,
    by_id: &BTreeMap<&str, &Task>,
    seen: &mut BTreeSet<String>,
    indent: usize,
    out: &mut String,
) {
    let Some(task) = by_id.get(id) else {
        return;
    };
    let mut deps: Vec<&String> = task.depends_on.iter().collect();
    deps.sort();
    for dep in deps {
        if !seen.insert(dep.clone()) {
            continue;
        }
        let title = by_id
            .get(dep.as_str())
            .map(|t| t.title.as_str())
            .unwrap_or("(missing)");
        out.push_str(&format!(
            "{:indent$}- {dep}  {title}\n",
            "",
            indent = indent
        ));
        walk_up(dep, by_id, seen, indent + 2, out);
    }
}

fn walk_down(
    id: &str,
    dependents: &BTreeMap<String, BTreeSet<String>>,
    by_id: &BTreeMap<&str, &Task>,
    seen: &mut BTreeSet<String>,
    indent: usize,
    out: &mut String,
) {
    let Some(children) = dependents.get(id) else {
        return;
    };
    for child in children {
        if !seen.insert(child.clone()) {
            continue;
        }
        let title = by_id
            .get(child.as_str())
            .map(|t| t.title.as_str())
            .unwrap_or("(missing)");
        out.push_str(&format!(
            "{:indent$}- {child}  {title}\n",
            "",
            indent = indent
        ));
        walk_down(child, dependents, by_id, seen, indent + 2, out);
    }
}

fn build_reverse(tasks: &[Task]) -> BTreeMap<String, BTreeSet<String>> {
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for t in tasks {
        for dep in &t.depends_on {
            map.entry(dep.clone()).or_default().insert(t.id.clone());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Size, Status};

    fn task(id: &str, deps: &[&str]) -> Task {
        Task {
            id: id.to_string(),
            title: format!("title-{id}"),
            size: Size::S,
            status: if deps.is_empty() {
                Status::Ready
            } else {
                Status::Blocked
            },
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            priority: None,
            commit: None,
            body: String::new(),
        }
    }

    #[test]
    fn upward_shows_transitive_deps() {
        let tasks = vec![task("T1", &[]), task("T2", &["T1"]), task("T3", &["T2"])];
        let out = render(&tasks, "T3");
        assert!(out.contains("T2"));
        assert!(out.contains("T1"));
    }

    #[test]
    fn downward_shows_transitive_dependents() {
        let tasks = vec![task("T1", &[]), task("T2", &["T1"]), task("T3", &["T2"])];
        let out = render(&tasks, "T1");
        let t2 = out.find("T2").unwrap();
        let t3 = out.find("T3").unwrap();
        assert!(t2 < t3, "T2 should appear before its descendant T3");
    }

    #[test]
    fn reports_none_when_leaf() {
        let tasks = vec![task("T1", &[])];
        let out = render(&tasks, "T1");
        assert!(out.contains("depends on:\n    (none)"));
        assert!(out.contains("dependents:\n    (none)"));
    }

    #[test]
    fn cycle_does_not_loop_forever() {
        // Synthetic: T1 depends on T2, T2 depends on T1. Render must terminate.
        let tasks = vec![task("T1", &["T2"]), task("T2", &["T1"])];
        let out = render(&tasks, "T1");
        assert!(out.contains("T2"));
    }

    #[test]
    fn missing_dep_marked() {
        let tasks = vec![task("T1", &["T99"])];
        let out = render(&tasks, "T1");
        assert!(out.contains("T99"));
        assert!(out.contains("(missing)"));
    }
}
