---
id: T14
title: backlog-status.sh bash fallback script
size: S
status: ready
priority: 2
---

## Sub-steps

- [ ] Create `.claude/hooks/backlog-status.sh` — pure bash, no external deps beyond coreutils + grep
- [ ] Parse `backlog/*.md` frontmatter: count ready/blocked/draft, identify next ready task (lowest priority then ID)
- [ ] Print a compact block suitable for SessionStart context injection (≤10 lines)
- [ ] Exit 0 on empty backlog, print "no tasks"
- [ ] `chmod +x` the script
- [ ] Document in script header: "Temporary — swap to `ether-forge status` once T6 lands"
