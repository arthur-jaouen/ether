---
id: T7
title: ether-forge check subcommand
size: S
status: ready
priority: 4
---

## Sub-steps

- [x] Implement `check` in `crates/ether-forge/src/cmd/check.rs` — runs `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check` in sequence
- [x] Stream child-process output live; exit non-zero on first failure
- [x] Unit test: synthetic `cargo` stub verifies command assembly and failure propagation
