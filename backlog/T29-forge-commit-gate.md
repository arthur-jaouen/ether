---
id: T29
title: Forge commit gate on reviewer blockers
size: M
status: ready
priority: 1
---

Make `ether-forge commit` refuse to commit when the reviewer artifact for the current task lists any blockers. Covers ROADMAP 0.5.5.3b.

## Sub-steps

- [x] Extend `ether-forge commit` to auto-discover `target/.ether-forge/review-T<n>.json` for the task ID being committed.
- [x] If `blockers` is non-empty, print the listing and exit nonzero.
- [x] Add `--force-review` flag as an escape hatch; when used, append a `Reviewed-by-force: true` trailer to the commit message.
- [x] Unit or integration test: fixture artifact with one blocker → commit refused; empty blockers → commit proceeds; `--force-review` → commit proceeds with trailer.
- [x] Update `.claude/skills/dev/SKILL.md` step 16 to pass the worktree path and task ID only (agent resolves context via `ether-forge task --context`); step 18's existing `ether-forge commit T<n> -a` automatically picks up the gate.
- [x] Run `ether-forge check` before committing.
