---
id: T8
title: ether-forge done subcommand with cascade rule
size: M
status: blocked
depends_on:
  - T5
---

## Sub-steps

- [ ] Implement `done T<n> [--commit <sha>]` in `crates/ether-forge/src/cmd/done.rs`
- [ ] Load task, set `status: done`, write `commit` field, move file to `backlog/done/`
- [ ] Cascade: scan all remaining tasks, remove completed ID from `depends_on`; if list empty, drop the field and flip `blocked` → `ready`
- [ ] Preserve markdown body byte-for-byte during frontmatter rewrite
- [ ] Refuse to run if the task is already `done` or has unsatisfied `depends_on`
- [ ] Integration test against a fixture backlog with chained dependencies
