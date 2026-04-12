---
id: T17
title: Rewrite ether-forge check for lean output and nextest
size: S
status: ready
priority: 4
---

## Sub-steps

- [ ] Rewrite `crates/ether-forge/src/cmd/check.rs` to run `cargo clippy --workspace --all-targets --message-format=short -q -- -D warnings` with `CARGO_TERM_COLOR=never`
- [ ] Chain `cargo nextest run --workspace --failure-output=final --status-level=fail --hide-progress-bar` with fail-fast (abort if clippy exits non-zero)
- [ ] Append `cargo test --doc --workspace` as a third step to cover the nextest doctest gap
- [ ] Document the `cargo install cargo-nextest` prerequisite in `crates/ether-forge/README.md` (create if missing)
- [ ] Update existing `check` unit/integration tests to match the new command invocation
