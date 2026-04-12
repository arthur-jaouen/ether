---
id: T14
title: backlog-status.sh bash fallback script
size: S
status: ready
priority: 2
---

## Sub-steps

- [x] Create `.claude/hooks/backlog-status.sh` — pure bash, no external deps beyond coreutils + grep
- [x] Parse `backlog/*.md` frontmatter: count ready/blocked/draft, identify next ready task (lowest priority then ID)
- [x] Print a compact block suitable for SessionStart context injection (≤10 lines)
- [x] Exit 0 on empty backlog, print "no tasks"
- [x] `chmod +x` the script
- [x] Document in script header: "Temporary — swap to `ether-forge status` once T6 lands"
