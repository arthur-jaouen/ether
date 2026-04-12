---
id: T20
title: ether-forge diff subcommand (review-scoped worktree diff)
size: S
status: ready
priority: 14
---

## Sub-steps

- [ ] Implement `diff [T<n>]` in `crates/ether-forge/src/cmd/diff.rs` — runs `git diff main` from the task's worktree (or current dir if no ID), strips lockfiles (`Cargo.lock`, `**/*.lock`), size-caps output with a truncation marker
- [ ] Wire into `crates/ether-forge/src/cmd/mod.rs` and the clap subcommand enum
- [ ] Integration test against a fixture worktree: assert lockfile exclusion, truncation marker when oversized, and exit code on non-existent task ID
