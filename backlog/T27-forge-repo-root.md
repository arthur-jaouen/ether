---
id: T27
title: "ether-forge: resolve repo root via git rev-parse --show-toplevel"
size: S
status: ready
---

# ether-forge: resolve repo root via git rev-parse --show-toplevel

Make `ether-forge` robust to being invoked from any subdirectory of the repo (including nested worktree paths). Today every subcommand defaults `--backlog = "backlog"` relative to cwd, and `cmd/worktree.rs:32` uses `current_dir()` as the repo root. A helper that shells to `git rev-parse --show-toplevel` once and threads the result through the CLI removes the cwd dependency.

Independent of T24/T25/T26 — those fix the worktree drift at the skill layer; this one keeps forge correct on its own.

## Sub-steps

- [ ] Add a `repo_root()` helper in `crates/ether-forge/src/main.rs` (or a new `repo.rs`) that runs `git rev-parse --show-toplevel` from `std::env::current_dir()` and returns the resolved `PathBuf`; error cleanly if not in a git repo.
- [ ] Change the `--backlog` default in `crates/ether-forge/src/main.rs` from the literal `"backlog"` to `<repo_root>/backlog`, computed at runtime before clap parsing or via a `default_value_t` closure.
- [ ] Update `crates/ether-forge/src/cmd/worktree.rs:32` and `crates/ether-forge/src/cmd/diff.rs:21` to use `repo_root()` instead of `current_dir()`.
- [ ] Add an integration test in `crates/ether-forge/tests/` that runs a forge subcommand from a subdirectory of a temp git repo and asserts it still finds `backlog/`.
- [ ] Run `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`.
