---
id: T38
title: ether-forge merge subcommand for skill wrap-up
size: L
status: ready
priority: 3
---

Skill wrap-up (step 23 of `/dev` and equivalents in `/groom`, `/roadmap`) strings together four raw git calls that each have a known failure mode: ff-merge refuses when `main` advanced during the session, `git worktree remove` errors when the directory is already gone, and `git branch -d` refuses on "not fully merged" branches with identical content. Collapse the dance into one forge primitive — the exit-side mirror of `preflight`.

See ROADMAP Phase 0.5.7 for the full spec.

## Sub-steps

- [ ] Add `Merge { id, keep, force_review, worktree }` variant to the clap `Command` enum in `crates/ether-forge/src/main.rs`.
- [ ] New `crates/ether-forge/src/cmd/merge.rs`: resolve worktree path (from `--worktree` or `git rev-parse --show-toplevel`), verify clean, fetch main, rebase if behind, re-run `check`, ff-merge into main, remove the worktree directory (ignoring `NotFound`), `git worktree prune`, delete the branch.
- [ ] Reuse the review-gate helpers from `cmd/commit.rs` (`load_artifact` + `evaluate_gate`) so blocker enforcement stays single-sourced. Promote them to `pub(crate)` if needed.
- [ ] `--keep` flag: perform the rebase + ff-merge but leave the worktree directory and branch in place.
- [ ] Unit tests: pure-function tests for "is behind main?" and "worktree clean?" predicates over injected git-runner closures (match the existing pattern in `preflight.rs`).
- [ ] Integration test under `crates/ether-forge/tests/`: spawn a throwaway repo + worktree, drive the happy path end-to-end, assert the worktree directory is gone and the branch is deleted.
- [ ] Regression test: simulate `main` advancing mid-session (extra commit on main after branch creation), confirm `merge` rebases and ff-merges cleanly.
- [ ] Regression test: pre-removed worktree directory — `merge` must still delete the branch and succeed.
- [ ] Update `.claude/skills/dev/SKILL.md` step 23: replace the `git merge --ff-only` / `git worktree remove` / `git branch -d` sequence with a single `ether-forge merge T<n>` call, still gated behind the `AskUserQuestion` confirmation.
- [ ] Update `.claude/skills/groom/SKILL.md` and `.claude/skills/roadmap/SKILL.md` wrap-up sections to call `ether-forge merge` the same way.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` stay green.
