---
id: T34
title: Remove ether-forge worktree subcommand
size: M
status: done
priority: 3
commit: 344617a
---

`ether-forge worktree T<n>` is dead code inside every agent-driven skill: only `EnterWorktree` re-roots Glob/Grep/Read/Edit, so no skill ever calls the CLI version. Rather than keep a shell-only fallback nobody uses, delete it.
