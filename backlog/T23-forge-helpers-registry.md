---
id: T23
title: ether-forge helpers subcommand (shared test helper registry)
size: S
status: ready
priority: 17
---

## Sub-steps

- [x] Implement `helpers` in `crates/ether-forge/src/cmd/helpers.rs` — scans `crates/*/tests/common/mod.rs` and emits function names with their owning crate, sorted deterministically
- [x] Wire into `cmd/mod.rs` and clap
- [x] Unit test against a workspace fixture: two crates with overlapping helper names produce a duplicate-highlighting output
- [x] Document the intended consumer (review agent duplication check) in the subcommand's `--help` text
