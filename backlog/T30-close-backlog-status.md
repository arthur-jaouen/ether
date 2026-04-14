---
id: T30
title: /close includes backlog status in wrap-up
size: S
status: ready
priority: 2
---

Wire `ether-forge status` into the `/close` skill so every session wrap-up ends with a concrete backlog delta instead of prose. Part of Phase 0.5.6 — analysis skills should query `ether-forge` as a shared state layer, not re-read files.

## Sub-steps

- [ ] Edit `.claude/skills/close/SKILL.md`: add `ether-forge status` to the final report step.
- [ ] Include output verbatim in the session summary (counts by status + next ready task).
- [ ] Manual smoke test: run `/close` at end of a session and confirm the status block appears.
