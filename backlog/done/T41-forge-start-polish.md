---
id: T41
title: ether-forge start — polish and skill wiring
size: L
status: done
priority: 1
commit: 5c6b556
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
