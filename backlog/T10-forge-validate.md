---
id: T10
title: ether-forge validate subcommand
size: M
status: blocked
depends_on:
  - T5
---

## Sub-steps

- [ ] Implement `validate` in `crates/ether-forge/src/cmd/validate.rs` — load all tasks from `backlog/` and `backlog/done/`
- [ ] Check: duplicate IDs across active and done
- [ ] Check: every `depends_on` ID exists; no self-dependency; no cycles (DFS)
- [ ] Check: `blocked` ↔ `depends_on` consistency (one requires the other)
- [ ] Check: `done` tasks have a `commit` field; active tasks do not
- [ ] Check: file name matches `T<id>-<slug>.md` pattern
- [ ] Exit non-zero with grouped error report; exit zero and print "OK" on clean state
- [ ] Unit tests covering each failure mode with minimal fixtures
