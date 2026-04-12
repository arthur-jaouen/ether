---
id: T14
title: backlog-status.sh bash fallback script
size: S
status: done
priority: 2
commit: 4a63e85
---

Pure-bash SessionStart fallback at `.claude/hooks/backlog-status.sh`.
Parses `backlog/*.md` YAML frontmatter, counts ready/blocked/draft,
and emits the next ready task by (priority asc, id asc). To be
replaced by `ether-forge status` once T6 lands.
