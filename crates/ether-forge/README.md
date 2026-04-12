# ether-forge

Development process CLI for the Ether ECS workspace. Manages the backlog
(`list`, `next`, `get`, `search`, `deps`, `status`) and runs workspace
verification (`check`).

## Prerequisites

`ether-forge check` shells out to `cargo-nextest` for the test run:

```bash
cargo install cargo-nextest
```

Without it, the `check` command will fail at the nextest step.

## `check`

Runs three cargo steps in order, aborting on the first failure:

1. `cargo clippy --workspace --all-targets --message-format=short -q -- -D warnings`
2. `cargo nextest run --workspace --failure-output=final --status-level=fail --hide-progress-bar`
3. `cargo test --doc --workspace` (doctests — nextest does not run them)

Each invocation is spawned with `CARGO_TERM_COLOR=never` so output is
grep-friendly.
