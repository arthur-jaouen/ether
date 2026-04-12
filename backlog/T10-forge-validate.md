---
id: T10
title: ether-forge validate subcommand
size: M
status: ready
priority: 5
---

## Sub-steps

- [x] Implement `validate` in `crates/ether-forge/src/cmd/validate.rs` — load all tasks from `backlog/` and `backlog/done/`
- [x] Check: duplicate IDs across active and done
- [x] Check: every `depends_on` ID exists; no self-dependency; no cycles (DFS)
- [x] Check: `blocked` ↔ `depends_on` consistency (one requires the other)
- [x] Check: `done` tasks have a `commit` field; active tasks do not
- [x] Check: file name matches `T<id>-<slug>.md` pattern
- [x] Exit non-zero with grouped error report; exit zero and print "OK" on clean state
- [x] Unit tests covering each failure mode with minimal fixtures
