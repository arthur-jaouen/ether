---
id: T39
title: ether-forge start subcommand for skill kickoff
size: L
status: draft
---

Mirror of T38 (`merge`) on the entry side. Today the `/dev` startup runs ~6 separate steps before any code is touched: `get T<n>`, `check`, `preflight --task T<n>`, `git worktree add`, orient, `git fetch` + rebase if behind. Collapse them into one `ether-forge start T<n>` primitive so `start` and `merge` become the bookends of every skill session.

## Sub-steps

- [ ] Add `Start { id, keep_existing }` variant to the clap `Command` enum in `crates/ether-forge/src/main.rs`.
- [ ] New `crates/ether-forge/src/cmd/start.rs`: load task, assert `status: ready`, run `check` + `preflight` logic in-process (reuse the existing modules — promote helpers to `pub(crate)` if needed), create or reuse worktree at `.claude/worktrees/dev-T<n>` on branch `worktree-dev-T<n>`, fetch `main`, rebase if behind.
- [ ] Output: print resolved worktree path, branch name, and a one-line "next: cd <path>" hint. Machine-readable enough that the skill can `cd "$(ether-forge start T<n> --print-path)"`.
- [ ] `--keep-existing` flag: if the worktree dir already exists, reuse it instead of erroring (matches the rerun-after-interruption case).
- [ ] Unit tests: pure-function tests for "is behind main?" and "worktree exists?" predicates over injected git-runner closures (match the pattern in `preflight.rs` and the new `merge.rs`).
- [ ] Integration test under `crates/ether-forge/tests/`: throwaway repo, drive `start T<n>` end-to-end, assert worktree dir + branch exist and HEAD matches main.
- [ ] Regression test: `main` advanced after the branch was first created — `start` must rebase cleanly.
- [ ] Regression test: pre-existing worktree dir with `--keep-existing` — must reuse, not error.
- [ ] Update `.claude/skills/dev/SKILL.md` startup section: replace the get/check/preflight/worktree/rebase sequence with a single `ether-forge start T<n>` call.
- [ ] Update `.claude/skills/groom/SKILL.md` and `.claude/skills/roadmap/SKILL.md` kickoff sections to call `ether-forge start` the same way (where they create worktrees).
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` stay green.
