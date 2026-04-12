---
id: T16
title: Claude hooks — SessionStart status and SessionEnd validate
size: S
status: ready
priority: 9
---

## Sub-steps

- [ ] Add `hooks.SessionStart` entry in `.claude/settings.json` running `.claude/hooks/backlog-status.sh` and injecting stdout as context
- [ ] Add `hooks.SessionEnd` entry running `.claude/hooks/validate.sh` (bash fallback) — minimal ID-uniqueness + depends_on existence checks
- [ ] Document in `.claude/settings.json` comment or adjacent README that both hook commands will be swapped to `ether-forge status` / `ether-forge validate` after T6 and T10
- [ ] Manual test: start a fresh session and confirm backlog status appears in context; end it and confirm validate runs
