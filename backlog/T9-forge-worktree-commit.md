---
id: T9
title: ether-forge worktree and commit subcommands
size: M
status: ready
priority: 7
---

## Sub-steps

- [x] `worktree T<n>` in `crates/ether-forge/src/cmd/worktree.rs` — run `git worktree add worktrees/T<n>-<slug> -b task/T<n> main`, print the absolute path
- [x] Derive slug from task title (lowercase, alphanumeric + hyphens)
- [x] Refuse if the worktree or branch already exists
- [x] `commit T<n>` in `crates/ether-forge/src/cmd/commit.rs` — run `ether-forge check` first, then `git commit` with message `T<n>: <title>` pulled from frontmatter
- [x] Pass through additional `git commit` args (e.g. `-a`, extra message lines)
- [x] Integration test: mock git invocations, verify argument assembly
