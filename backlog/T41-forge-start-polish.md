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

## Design: machine-readable status line

`ether-forge start` cannot tell the harness to switch its tool target — only
`EnterWorktree` can do that. So the skill still has to decide, after each
`start` call, whether to follow up with `EnterWorktree`. To avoid prose-
matching English output or duplicating the "does the dir exist?" logic, each
`start` invocation emits a final stable sentinel line on stdout:

```
start: mode=created path=<absolute-path> branch=<branch-name>
```
or
```
start: mode=in-place branch=<current-branch>
```

Skills grep for `mode=created` to decide whether to call `EnterWorktree`.
The sentinel is the contract — every other stdout line is human-readable and
may change without breaking callers. Unit-test the two emission paths so the
format can't drift.

## Sub-steps

- [ ] `--branch <name>` mode in `crates/ether-forge/src/cmd/start.rs`: factor
  the core flow so it accepts an already-resolved `(worktree_dir, branch_name,
  task_id: Option<&str>)` triple, then have the T<n> path and the `--branch`
  path both call it. Preflight runs without `--task` in branch mode.
- [ ] **In-place fallback**: before attempting `git worktree add`, inspect
  `git worktree list --porcelain`. If the primary entry's branch is a
  non-main feature branch, short-circuit to the in-place path: run preflight
  (no claim check), run `check`, emit the `mode=in-place` sentinel (see
  below), return `Ok(())`. Reuse `cmd::merge::in_place_branch` as the shared
  pure predicate (move it to a neutral helper module if either crate grows a
  third caller). Refuses with a clear error if the current branch name
  conflicts with the requested task id or `--branch` value (e.g. `start T40`
  on `dev-T17` should not silently succeed).
- [ ] **Status-line sentinel**: emit exactly one final stdout line per
  invocation in the stable machine-readable format
  `start: mode=created path=<abs> branch=<name>` (happy path) or
  `start: mode=in-place branch=<name>` (fallback). Factor into a tiny helper
  so both paths call the same formatter; unit-test both emission shapes by
  capturing stdout. This is the contract the three skills rely on for
  conditional `EnterWorktree` dispatch.
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
- [ ] Regression test (sentinel format): unit-test the status-line helper
  directly for both shapes (`mode=created path=… branch=…` and
  `mode=in-place branch=…`) AND assert the integration-test stdout from at
  least one happy-path case and one in-place case ends with the expected
  sentinel. The format is a load-bearing contract for the three skills, so
  breaking it must fail tests.
- [ ] Update `.claude/skills/dev/SKILL.md` startup section: replace the
  `get` + `check` + `preflight` + `EnterWorktree` + fetch/rebase sequence
  (steps 8–10 in the "Fresh" state) with a single `ether-forge start T<n>`
  call followed by a conditional `EnterWorktree dev-T<n>` that fires only
  when the `start` stdout ends with `mode=created`. **Delete the "Fresh vs
  already-on-branch" session-layout table** at the top of SKILL.md — the
  skill now has one call path and the binary's sentinel decides. Also
  delete the stale "Skip if already on a feature branch" prose in the
  preflight guidance.
- [ ] Update `.claude/skills/groom/SKILL.md` kickoff: replace step 10's
  `ether-forge preflight` with
  `ether-forge start --branch groom-$(date +%Y-%m-%d)`, and rewrite step 11
  so `EnterWorktree groom-<date>` is called only when the `start` output
  contained `mode=created`. Delete the "skip if already inside a worktree"
  prose in both steps — the sentinel absorbs that decision. Keep the rest
  of the session shape (apply auto-fixes, validate, commit) untouched.
- [ ] Update `.claude/skills/roadmap/SKILL.md` kickoff: identical rewrite
  with `ether-forge start --branch roadmap-$(date +%Y-%m-%d)` and a
  conditional `EnterWorktree roadmap-<date>` in step 11.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
  stay green.

## Splittability note

This is L because it touches 2 Rust files + 3 skill docs and adds 7 new
regression tests (including the sentinel-format lock-in). A future groomer
may split it into T41a (Rust: dual-mode + in-place fallback + sentinel
helper + `--keep-existing` + all regression tests) and T41b (skill-wiring
docs for `/dev`, `/groom`, `/roadmap` — each gaining a conditional
`EnterWorktree` call gated on the `mode=created` sentinel), with T41b
depending on T41a. Kept as one task for now so the `start`/`merge` symmetry
and the skill-side table collapse land atomically — a half-migrated skill
fleet is a worse intermediate state than a single larger landing.
