---
id: T24
title: "/groom skill: migrate to EnterWorktree"
size: S
status: ready
priority: 1
---

# /groom skill: migrate to EnterWorktree

Replace the manual `git worktree add` + `cd` dance in `/groom` with the `EnterWorktree` harness primitive so every tool (not just Bash) targets the worktree.

## Sub-steps

- [x] Edit `.claude/skills/groom/SKILL.md` Phase 5 (step 10): replace the `git worktree add worktrees/groom-YYYY-MM-DD -b groom-YYYY-MM-DD main && cd …` block with an `EnterWorktree` call named `groom-YYYY-MM-DD`.
- [x] Replace step 14's manual merge + cleanup flow with `ExitWorktree` (`action: keep` pre-merge; `action: remove` after a successful `git merge --ff-only` from `main`).
- [x] Add a one-line fallback note: if already inside a worktree, skip `EnterWorktree` and work in place (the primitive refuses nesting).
- [x] Dry-run the skill on a trivial groom scope to confirm Glob/Grep/Read/Edit resolve against the worktree.
