---
id: T23
title: ether-forge helpers subcommand (shared test helper registry)
size: M
status: ready
priority: 17
---

## Sub-steps

- [ ] Implement `helpers` in `crates/ether-forge/src/cmd/helpers.rs` — scans `crates/*/tests/common/mod.rs` and emits function names with their owning crate, sorted deterministically
- [ ] Wire into `cmd/mod.rs` and clap
- [ ] Unit test against a workspace fixture: two crates with overlapping helper names produce a duplicate-highlighting output
- [ ] Document the intended consumer (review agent duplication check) in the subcommand's `--help` text
