---
id: T5
title: Scaffold ether-forge crate and frontmatter parser
size: M
status: ready
priority: 1
---

## Sub-steps

- [ ] Create `crates/ether-forge/Cargo.toml` — binary crate, deps: `clap` (derive), `serde`, `serde_yaml`, `anyhow`
- [ ] Add `ether-forge` to workspace members in root `Cargo.toml`
- [ ] Create `crates/ether-forge/src/main.rs` — clap `Cli` struct with empty `Subcommand` enum stub
- [ ] Create `crates/ether-forge/src/task.rs` — `Task` struct (id, title, size, status, depends_on, priority, commit) with `serde::Deserialize`
- [ ] Implement `Task::load(path: &Path) -> Result<Task>` — splits YAML frontmatter from markdown body, parses with `serde_yaml`
- [ ] Implement `Task::load_all(dir: &Path) -> Result<Vec<Task>>` — deterministic sort by ID
- [ ] Unit tests: parse valid frontmatter, reject malformed, round-trip a known task file
