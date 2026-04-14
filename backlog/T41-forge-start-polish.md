---
id: T41
title: ether-forge start — polish and skill wiring
size: L
status: ready
---

Follow-up to T40. Harden the `start` subcommand's edge cases, add a
branch-name mode so it covers `/groom` and `/roadmap` (not just `/dev`), and
swap all three skills over to it — so `start` / `merge` become the symmetric
bookends of every worktree-creating session. This is the entry-side mirror of
`merge`'s existing `TaskId | BranchName` dual mode.

## Design: dual mode

- `ether-forge start T<n>` — task mode (T40 shape). Loads the task, asserts
  `status: ready`, creates `.claude/worktrees/dev-T<n>` on branch
  `worktree-dev-T<n>`.
- `ether-forge start --branch <name>` — branch mode (new). Skips the backlog
  lookup and the ready-status assertion. Creates `.claude/worktrees/<name>` on
  branch `<name>`. Used by `/groom` (`groom-YYYY-MM-DD`) and `/roadmap`
  (`roadmap-YYYY-MM-DD`), which have no task id to claim.

Both modes share the same `preflight` + `check` + `git worktree add` + fetch
+ rebase machinery. The clap surface is `start { id: Option<String>, branch:
Option<String> }` with `conflicts_with`/`required_unless_present_any` so
exactly one is supplied.

## Sub-steps

- [ ] `--branch <name>` mode in `crates/ether-forge/src/cmd/start.rs`: factor
  the core flow so it accepts an already-resolved `(worktree_dir, branch_name,
  task_id: Option<&str>)` triple, then have the T<n> path and the `--branch`
  path both call it. Preflight runs without `--task` in branch mode.
- [ ] `--keep-existing` flag: if the target worktree dir already exists on
  disk, reuse it (`git worktree add` becomes a no-op verification) instead of
  erroring. Applies to both modes — handles the rerun-after-interruption case.
- [ ] Regression test: `main` advanced after the worktree's branch was first
  created — `start` fetches origin/main and rebases the worktree cleanly.
  Requires a local bare remote in the fixture since the current T40 fixtures
  have no `origin`.
- [ ] Regression test: pre-existing worktree dir with `--keep-existing` — must
  reuse, not error. Without the flag, must still error clearly.
- [ ] Regression test: `ether-forge start --branch groom-2026-04-14` happy
  path against a throwaway repo — asserts worktree dir, branch, and HEAD
  match main.
- [ ] Update `.claude/skills/dev/SKILL.md` startup section: replace the
  `get` + `check` + `preflight` + `EnterWorktree` + fetch/rebase sequence
  (steps 8–10 in the "Fresh" state) with a single `ether-forge start T<n>`
  call. Remove the now-redundant mention of `ether-forge preflight --task`.
- [ ] Update `.claude/skills/groom/SKILL.md` kickoff: replace step 10's
  `ether-forge preflight` + manual `git worktree add` with
  `ether-forge start --branch groom-$(date +%Y-%m-%d)`.
- [ ] Update `.claude/skills/roadmap/SKILL.md` kickoff: replace step 10 with
  `ether-forge start --branch roadmap-$(date +%Y-%m-%d)`.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
  stay green.

## Splittability note

This is L because it touches 2 Rust files + 3 skill docs and adds 3 new
regression tests. A future groomer may split it into T41a (Rust dual-mode +
`--keep-existing` + tests) and T41b (skill-wiring docs), with T41b depending
on T41a. Kept as one task for now so the `start` / `merge` symmetry lands
atomically — a half-migrated skill fleet is a worse intermediate state than
a single larger landing.
