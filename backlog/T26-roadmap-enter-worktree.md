---
id: T26
title: "/roadmap skill: migrate to EnterWorktree"
size: S
status: ready
priority: 1
---

# /roadmap skill: migrate to EnterWorktree

Replace manual `git worktree add` + `cd` in `/roadmap` Phase 3 with `EnterWorktree`, so ROADMAP.md edits land in the worktree instead of the parent checkout.

## Sub-steps

- [x] Edit `.claude/skills/roadmap/SKILL.md` step 8: replace the `git worktree add worktrees/roadmap-<topic> -b roadmap/<topic> main` block with `EnterWorktree` named `roadmap-<topic>`.
- [x] Replace step 13's cleanup with `ExitWorktree` (`keep` pre-merge; `remove` after ff-merge into `main`).
- [x] Add a one-line fallback note: if already inside a worktree, skip `EnterWorktree` and edit in place.
- [x] Dry-run a trivial roadmap edit to confirm the Edit tool writes to the worktree path.
