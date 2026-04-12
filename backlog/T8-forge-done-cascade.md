---
id: T8
title: ether-forge done subcommand with cascade rule
size: M
status: ready
priority: 3
---

## Sub-steps

- [x] Implement `done T<n> [--commit <sha>]` in `crates/ether-forge/src/cmd/done.rs`
- [x] Load task, set `status: done`, write `commit` field, move file to `backlog/done/`
- [x] Cascade: scan all remaining tasks, remove completed ID from `depends_on`; if list empty, drop the field and flip `blocked` → `ready`
- [x] Preserve markdown body byte-for-byte during frontmatter rewrite
- [x] Refuse to run if the task is already `done` or has unsatisfied `depends_on`
- [x] Integration test against a fixture backlog with chained dependencies
