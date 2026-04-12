---
id: T24
title: "/groom skill: migrate to EnterWorktree"
size: S
status: done
priority: 1
commit: 57e93f6
---

# /groom skill: migrate to EnterWorktree

Replace the manual `git worktree add` + `cd` dance in `/groom` with the `EnterWorktree` harness primitive so every tool (not just Bash) targets the worktree.
