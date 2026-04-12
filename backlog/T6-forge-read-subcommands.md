---
id: T6
title: ether-forge read subcommands (list, next, get, search, deps, status)
size: M
status: ready
priority: 6
---

## Sub-steps

- [ ] `list [--status <filter>]` in `crates/ether-forge/src/cmd/list.rs` — tabular output sorted by priority then ID
- [ ] `next` in `crates/ether-forge/src/cmd/next.rs` — print top ready task (priority-first, lowest ID tiebreaker)
- [ ] `get T<n>` in `crates/ether-forge/src/cmd/get.rs` — full task detail (frontmatter + sub-steps)
- [ ] `search <query>` in `crates/ether-forge/src/cmd/search.rs` — case-insensitive match on title, ID, sub-step text
- [ ] `deps T<n>` in `crates/ether-forge/src/cmd/deps.rs` — print dependency tree upward and dependents downward
- [ ] `status` in `crates/ether-forge/src/cmd/status.rs` — summary counts (ready/blocked/draft/done), next up, blocked list
- [ ] Wire all subcommands into the clap enum in `main.rs`
- [ ] Integration tests against a fixture backlog directory under `crates/ether-forge/tests/fixtures/`
