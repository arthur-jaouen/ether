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

## Design: in-place fallback (symmetry with `merge`)

`ether-forge merge <target>` already falls back to an in-place merge when no
linked worktree claims the target (primary worktree on a feature branch,
e.g. Claude Code on the Web scaffolding branches, resumed `/dev` sessions).
`start` grows the entry-side mirror so the two primitives are fully
symmetric and the skills stop needing a kickoff dispatch table.

Rule: if the primary worktree's HEAD is already on a non-main branch AND
the caller did not pass `--worktree`/`--keep-existing` for an explicit
opt-in, `start` prints `start: already on <branch>, skipping worktree
creation` and returns `Ok(())` after running `preflight` (without the
claim check) and `check`. It does not try to create a second nested
worktree, and it does not rename or reset the existing branch.

This collapses the `/dev` SKILL.md "Fresh vs already-on-branch" table into
a single call path — the skill always invokes `ether-forge start …`, and
the binary decides whether a worktree is needed.

## Sub-steps

- [ ] `--branch <name>` mode in `crates/ether-forge/src/cmd/start.rs`: factor
  the core flow so it accepts an already-resolved `(worktree_dir, branch_name,
  task_id: Option<&str>)` triple, then have the T<n> path and the `--branch`
  path both call it. Preflight runs without `--task` in branch mode.
- [ ] **In-place fallback**: before attempting `git worktree add`, inspect
  `git worktree list --porcelain`. If the primary entry's branch is a
  non-main feature branch, short-circuit to the in-place path: run preflight
  (no claim check), run `check`, print `start: already on <branch>, skipping
  worktree creation`, return `Ok(())`. Reuse `cmd::merge::in_place_branch` as
  the shared pure predicate (move it to a neutral helper module if either
  crate grows a third caller). Refuses with a clear error if the current
  branch name conflicts with the requested task id or `--branch` value
  (e.g. `start T40` on `dev-T17` should not silently succeed).
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
- [ ] Regression test (in-place fallback, task mode): set up a repo with the
  primary worktree checked out on `claude/scaffolding-xyz`, run `ether-forge
  start T<n>`, assert the worktree dir was NOT created, the branch stays put,
  and stdout contains the "already on" notice.
- [ ] Regression test (in-place fallback, branch mode): same fixture, run
  `ether-forge start --branch groom-2026-04-14` from the scaffolding branch,
  assert the same skip-and-return behavior.
- [ ] Regression test (conflict): on branch `dev-T17`, run `ether-forge start
  T40` — must refuse with a clear "current branch does not claim T40" error
  rather than silently no-opping.
- [ ] Update `.claude/skills/dev/SKILL.md` startup section: replace the
  `get` + `check` + `preflight` + `EnterWorktree` + fetch/rebase sequence
  (steps 8–10 in the "Fresh" state) with a single `ether-forge start T<n>`
  call. **Collapse the "Fresh vs already-on-branch" table**: the skill now
  always calls `start T<n>`; the binary decides. `EnterWorktree dev-T<n>`
  happens only if `start` actually created one (detect via stdout or by
  checking `.claude/worktrees/dev-T<n>` after the call).
- [ ] Update `.claude/skills/groom/SKILL.md` kickoff: replace step 10's
  `ether-forge preflight` + manual `git worktree add` with
  `ether-forge start --branch groom-$(date +%Y-%m-%d)`. Remove the "skip if
  already inside a worktree" prose — `start` absorbs that decision.
- [ ] Update `.claude/skills/roadmap/SKILL.md` kickoff: replace step 10 with
  `ether-forge start --branch roadmap-$(date +%Y-%m-%d)`. Remove the same
  prose.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
  stay green.

## Splittability note

This is L because it touches 2 Rust files + 3 skill docs and adds 6 new
regression tests. A future groomer may split it into T41a (Rust: dual-mode +
in-place fallback + `--keep-existing` + all regression tests) and T41b
(skill-wiring docs for `/dev`, `/groom`, `/roadmap`), with T41b depending on
T41a. Kept as one task for now so the `start`/`merge` symmetry and the
skill-side table collapse land atomically — a half-migrated skill fleet is a
worse intermediate state than a single larger landing.
