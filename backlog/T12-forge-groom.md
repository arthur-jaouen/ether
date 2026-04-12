---
id: T12
title: ether-forge groom subcommand (migrate lint and coverage logic)
size: L
status: ready
priority: 11
---

## Sub-steps

- [x] Implement `groom` in `crates/ether-forge/src/cmd/groom.rs` — reuses `validate` as its lint phase
- [x] Parse `ROADMAP.md` sections and map each to active/done tasks by keyword + explicit section tags
- [x] Classify each section: covered / partial / uncovered / done
- [x] Auto-cascade satisfied deps (same rule as `done`)
- [x] Emit a structured JSON report on `--json`, human-readable otherwise
- [x] Flag: stale sub-steps (referenced file/function no longer exists) via Glob + Grep over the workspace
- [x] Flag: size/sub-step count mismatch
- [x] Do not mutate backlog without `--apply`; default is dry-run reporting
- [x] Integration tests with a fixture ROADMAP + backlog directory covering each classification
