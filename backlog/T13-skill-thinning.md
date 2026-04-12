---
id: T13
title: Thin /dev, /groom, /backlog skills to shell out to ether-forge
size: M
status: ready
priority: 10
---

## Sub-steps

- [x] Rewrite `.claude/skills/backlog/SKILL.md` — list/add/done/status sections all shell out to `ether-forge`
- [x] Rewrite `.claude/skills/dev/SKILL.md` — pick task via `ether-forge next`, create worktree via `ether-forge worktree`, commit via `ether-forge commit`, mark done via `ether-forge done`
- [x] Rewrite `.claude/skills/groom/SKILL.md` — invoke `ether-forge groom --json`, present report, apply via `ether-forge groom --apply`
- [x] Remove inline bash task-parsing loops; keep skill prose focused on conversation flow
- [x] Manual test: run each skill end-to-end against the live backlog
