---
id: T13
title: Thin /dev, /groom, /backlog skills to shell out to ether-forge
size: M
status: blocked
depends_on:
  - T12
priority: 10
---

## Sub-steps

- [ ] Rewrite `.claude/skills/backlog/SKILL.md` — list/add/done/status sections all shell out to `ether-forge`
- [ ] Rewrite `.claude/skills/dev/SKILL.md` — pick task via `ether-forge next`, create worktree via `ether-forge worktree`, commit via `ether-forge commit`, mark done via `ether-forge done`
- [ ] Rewrite `.claude/skills/groom/SKILL.md` — invoke `ether-forge groom --json`, present report, apply via `ether-forge groom --apply`
- [ ] Remove inline bash task-parsing loops; keep skill prose focused on conversation flow
- [ ] Manual test: run each skill end-to-end against the live backlog
