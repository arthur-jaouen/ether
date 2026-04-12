---
id: T25
title: "/dev skill: migrate to EnterWorktree"
size: S
status: ready
priority: 1
---

# /dev skill: migrate to EnterWorktree

Replace manual `git worktree add` + `cd` in `/dev` with the `EnterWorktree` harness primitive so every tool targets the dev worktree, not main.

## Sub-steps

- [ ] Edit `.claude/skills/dev/SKILL.md`: replace the worktree creation step with `EnterWorktree` named `dev-T<n>` (derive `<n>` from the picked task ID).
- [ ] Replace the cleanup/merge step with `ExitWorktree` (`keep` if the user wants to review, `remove` after a successful ff-merge into `main`).
- [ ] Add a one-line fallback note: if already inside a worktree, skip `EnterWorktree` and work in place.
- [ ] Dry-run on a trivial `[ready]` task to confirm file reads/edits land in the dev worktree.
