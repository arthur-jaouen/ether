---
id: T31
title: /roadmap grounds session in ether-forge status preamble
size: S
status: ready
priority: 5
---

The `/roadmap` skill currently opens without querying backlog state — strategic edits can drift from what's actually ready, blocked, or in flight. Add a grounding preamble so every session starts from ground truth.

## Sub-steps

- [x] Edit `.claude/skills/roadmap/SKILL.md`: insert `ether-forge status` + `ether-forge list` as the first setup step.
- [x] Optional follow-up in same edit: document `ether-forge groom --json` (dry-run) as a way to see coverage drift vs `ROADMAP.md` before editing it.
- [x] Confirm the new preamble fits under the existing worktree setup (grounding runs before `EnterWorktree`, so it's read-only).
