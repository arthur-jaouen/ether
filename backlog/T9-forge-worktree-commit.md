---
id: T9
title: ether-forge worktree and commit subcommands
size: M
status: blocked
depends_on:
  - T5
  - T7
---

## Sub-steps

- [ ] `worktree T<n>` in `crates/ether-forge/src/cmd/worktree.rs` — run `git worktree add worktrees/T<n>-<slug> -b task/T<n> main`, print the absolute path
- [ ] Derive slug from task title (lowercase, alphanumeric + hyphens)
- [ ] Refuse if the worktree or branch already exists
- [ ] `commit T<n>` in `crates/ether-forge/src/cmd/commit.rs` — run `ether-forge check` first, then `git commit` with message `T<n>: <title>` pulled from frontmatter
- [ ] Pass through additional `git commit` args (e.g. `-a`, extra message lines)
- [ ] Integration test: mock git invocations, verify argument assembly
