---
id: T25
title: "/dev skill: migrate to EnterWorktree"
size: S
status: done
priority: 1
commit: 72433d9
---

# /dev skill: migrate to EnterWorktree

Replace manual `git worktree add` + `cd` in `/dev` with the `EnterWorktree` harness primitive so every tool targets the dev worktree, not main.
