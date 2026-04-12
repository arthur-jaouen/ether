# ether-forge

Development process CLI for the Ether ECS workspace. Manages the backlog
(`list`, `next`, `get`, `search`, `deps`, `status`), runs workspace
verification (`check`), and wraps `ast-grep` for structural search and
rewrites (`find`, `rewrite`).

## Prerequisites

`ether-forge check` shells out to `cargo-nextest` for the test run, and
`ether-forge find` / `rewrite` shell out to `ast-grep`:

```bash
cargo install cargo-nextest
cargo install ast-grep
```

Without `cargo-nextest` the `check` command will fail at the nextest step;
without `ast-grep` the `find` and `rewrite` commands will fail to spawn.

## `find` / `rewrite`

```bash
ether-forge find '$X.unwrap()'                 # search with an inline pattern
ether-forge find --rule no-unwrap-in-core      # resolve .claude/rules/sg/<name>.yml
ether-forge rewrite '$X.unwrap()' --to '$X.expect("todo")'
```

Rules live under `.claude/rules/sg/`; `--rule <name>` resolves
`<name>.yml` inside that directory.

## `check`

Runs three cargo steps in order, aborting on the first failure:

1. `cargo clippy --workspace --all-targets --message-format=short -q -- -D warnings`
2. `cargo nextest run --workspace --failure-output=final --status-level=fail --hide-progress-bar`
3. `cargo test --doc --workspace` (doctests — nextest does not run them)

Each invocation is spawned with `CARGO_TERM_COLOR=never` so output is
grep-friendly.
