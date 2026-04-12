---
id: T35
title: ether-forge preflight subcommand for worktree sessions
size: L
status: ready
priority: 2
---

Skill worktree sessions currently hit two preventable failures: (1) dirty `main` before `EnterWorktree` strands edits that break the later ff-merge, (2) a worktree base behind `main`'s HEAD forces a rebase dance at merge time. Both checks are identical across `/dev`, `/groom`, and `/roadmap`, so they belong in one forge primitive rather than copy-pasted into each skill.

## Sub-steps

- [ ] Add `ether-forge preflight [--task T<n>]` subcommand in `crates/ether-forge/src/main.rs`.
- [ ] Refuse with exit code 1 and a listing if `main`'s working tree is dirty (`git status --porcelain` on main).
- [ ] Refuse if the current branch's merge base with `main` is not `main`'s HEAD (worktree is behind).
- [ ] With `--task T<n>`: also refuse if a branch matching the task ID already exists (matches `/dev` step 3 stale-claim check).
- [ ] Unit tests for each failure mode using a scratch git repo fixture.
- [ ] Wire `ether-forge preflight` into `.claude/skills/dev/SKILL.md`, `.claude/skills/groom/SKILL.md`, and `.claude/skills/roadmap/SKILL.md` as the step immediately before `EnterWorktree`.
- [ ] Manual smoke: run `/groom` with a dirty `ROADMAP.md` on main and confirm preflight refuses before the worktree is created.
