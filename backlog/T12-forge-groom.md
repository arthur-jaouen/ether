---
id: T12
title: ether-forge groom subcommand (migrate lint and coverage logic)
size: L
status: blocked
depends_on:
  - T10
priority: 11
---

## Sub-steps

- [ ] Implement `groom` in `crates/ether-forge/src/cmd/groom.rs` — reuses `validate` as its lint phase
- [ ] Parse `ROADMAP.md` sections and map each to active/done tasks by keyword + explicit section tags
- [ ] Classify each section: covered / partial / uncovered / done
- [ ] Auto-cascade satisfied deps (same rule as `done`)
- [ ] Emit a structured JSON report on `--json`, human-readable otherwise
- [ ] Flag: stale sub-steps (referenced file/function no longer exists) via Glob + Grep over the workspace
- [ ] Flag: size/sub-step count mismatch
- [ ] Do not mutate backlog without `--apply`; default is dry-run reporting
- [ ] Integration tests with a fixture ROADMAP + backlog directory covering each classification
