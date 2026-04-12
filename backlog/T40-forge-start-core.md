---
id: T40
title: ether-forge start subcommand — core
size: M
status: draft
---

Entry-side mirror of T38 (`merge`). Ship a working `ether-forge start T<n>` that collapses the `/dev` kickoff dance (`get`, `check`, `preflight`, worktree add, fetch+rebase) into one primitive. This task covers the happy path only; edge-case flags and skill wiring land in the follow-up.

## Sub-steps

- [ ] Add `Start { id }` variant to the clap `Command` enum in `crates/ether-forge/src/main.rs`.
- [ ] New `crates/ether-forge/src/cmd/start.rs`: load task, assert `status: ready`, reuse `check` + `preflight` logic in-process, create worktree at `.claude/worktrees/dev-T<n>` on branch `worktree-dev-T<n>`, fetch `main`, rebase if behind. Print resolved path + branch + next-step hint.
- [ ] Unit tests: pure-function predicates (`is_behind_main`, `worktree_exists`) over injected git-runner closures — match the pattern in `preflight.rs` / `merge.rs`.
- [ ] Integration test under `crates/ether-forge/tests/`: throwaway repo, drive `start T<n>` happy path, assert worktree dir + branch exist and HEAD matches main.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` stay green.
