---
id: T16
title: Claude hooks — SessionStart status and SessionEnd validate
size: S
status: done
priority: 9
commit: 8a3cd79
---

Wired `SessionStart` to `.claude/hooks/backlog-status.sh` and `SessionEnd`
to a new `.claude/hooks/validate.sh` (bash fallback: ID uniqueness +
depends_on existence). Both scripts will be replaced by `ether-forge
status` / `ether-forge validate` after T6 and T10 land — see
`.claude/hooks/README.md`.
