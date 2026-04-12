---
id: T36
title: ether-forge task check batches sub-step checkoffs
size: M
status: draft
---

`/dev` step 13 ("check off each sub-step immediately") is the only remaining
task-file mutation that still goes through Read+Edit. Claude routinely tries
to Edit before reading, and cannot batch multiple checkoffs in one call.

Add `ether-forge task check <id> <idx>...` — 1-based indices against the
`## Sub-steps` list, variadic so N checkoffs happen in a single invocation.
Atomic read+write, no prior Read required.

## Sub-steps

- [ ] Add `task check` subcommand to `crates/ether-forge/src/cmd/task.rs` (or split into `cmd/task_check.rs`) accepting `<id>` and one or more 1-based indices
- [ ] Parse the task body, locate the `## Sub-steps` section, walk `- [ ]` / `- [x]` lines, flip the matching indices to `- [x]`
- [ ] Error cleanly on out-of-range index, missing `## Sub-steps` section, or already-checked line (warn but don't fail on the last)
- [ ] Wire the subcommand into `main.rs` clap enum
- [ ] Unit tests: single index, multiple indices, out-of-range, missing section, idempotent re-check
- [ ] Update `/dev` SKILL.md step 13 to call `ether-forge task check T<n> <idx>...` instead of Edit
- [ ] `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`
