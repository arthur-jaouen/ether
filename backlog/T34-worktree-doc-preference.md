---
id: T34
title: Remove ether-forge worktree subcommand
size: M
status: ready
priority: 3
---

`ether-forge worktree T<n>` is dead code inside every agent-driven skill: only `EnterWorktree` re-roots Glob/Grep/Read/Edit, so no skill ever calls the CLI version. Rather than keep a shell-only fallback nobody uses, delete it.

## Sub-steps

- [x] Grep `.claude/skills/` and `backlog/` to confirm zero callers of `ether-forge worktree`.
- [x] Remove the `Worktree` variant from the `Command` enum in `crates/ether-forge/src/main.rs` (or wherever the clap subcommand is declared).
- [x] Delete the handler and any helper functions that become unused.
- [x] Remove or update tests covering the subcommand.
- [x] Update `ROADMAP.md` Phase 0 "Lifecycle subcommands" section to strike the `worktree` bullet.
- [x] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` stay green.
