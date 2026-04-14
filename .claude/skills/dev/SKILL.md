---
name: dev
description: Autonomous development session on the Ether ECS workspace. Picks a backlog item, implements it, tests it, self-reviews, and commits.
argument-hint: [T<n> or empty for next ready]
---

# Ether Development Session

Work autonomously on the Ether ECS workspace at `/home/arthur/ether`. Lean on `ether-forge` for every backlog operation — it handles parsing, worktree creation, verification, and dependency cascades.

## Prerequisites

This skill uses deferred tools that are not resident by default. Before step 1, load them in a single `ToolSearch` call so they are available when the workflow needs them — otherwise you will stall mid-flow:

```
ToolSearch query="select:TodoWrite,AskUserQuestion"
```

`TodoWrite` tracks the sub-step checklist. `AskUserQuestion` is used at wrap-up time to confirm the merge.

## Session layout: fresh main vs already-on-branch

Two entry states are supported. Identify which one applies before step 8:

| State | How to detect | Branching strategy |
|-------|---------------|--------------------|
| **Fresh**, starting from `main` | `git branch --show-current` prints `main` | `preflight` → `EnterWorktree dev-T<n>` (default path, steps 8–10) |
| **Already on a feature branch** (Claude Code on the web pre-checks out a branch like `claude/implement-…`, or you resumed a session already on `dev-T<n>`) | `git branch --show-current` prints anything other than `main` | **Skip** `preflight` and `EnterWorktree`. Commit directly on the current branch. At wrap-up, merge manually (the `ether-forge merge T<n>` primitive only handles the `dev-T<n>` worktree layout). |

If the current branch is a stale `dev-T<m>` worktree belonging to a different, uncompleted task, stop and warn the user — do not hijack it.

## Setup

1. `cd /home/arthur/ether`
2. **Sync with upstream.** `git fetch origin main` so `ether-forge next` and `git log origin/main -- <path>` see the latest landed work. If the session-start hook's `next:` hint disagrees with `ether-forge next`, trust the hook — it ran after the fetch and your local backlog view may be stale.
3. **Task selection:**
   - If the user passed a `T<n>` argument, use that ID.
   - Otherwise, `ether-forge next` prints the top ready task (priority, then ID).
   - If no ready tasks exist, stop: "No ready tasks. Run `/groom` to generate them."
4. **Stale detection + claim check:** `git branch --list 'T*'`. If a branch already exists for the picked ID, skip to the next ready task (`ether-forge list --status ready`). For other `T*` branches with no worktree, warn the user.
5. `ether-forge get T<n>` — inspect the task body. If any sub-steps are already checked (`- [x]`), this is a **resumed session**; report which are done and start from the first unchecked step.
6. If the task needs more context, read the relevant `ROADMAP.md` section.
7. `CLAUDE.md` is already in the system context — no need to re-read.
8. `ether-forge check` — verifies fmt/clippy/tests in one call. If it fails, **first** run `git log origin/main -- <failing-path>` to see whether upstream already has a fix; if so, rebase/pull instead of patching locally. Otherwise fix the baseline, commit the fix, then continue.

## Claim + Isolate

9. **Fresh state only:** `ether-forge preflight --task T<n>` — refuses if `main` is dirty, the current branch is behind `main`, or a branch already claims the id. Fix whatever it reports before going further. Skip if you are already on a feature branch (the check is for the *pre-entry* environment).
10. **Fresh state only:** call `EnterWorktree` with `name: "dev-T<n>"` so every tool (Glob/Grep/Read/Edit/Bash) resolves against the isolated worktree. Skip if already on a feature branch — `EnterWorktree` refuses to nest, and it is pointless to create a `dev-T<n>` worktree when the harness has already placed you on a working branch.
11. All further work runs inside whichever branch is now current — the new worktree on the fresh path, or the pre-existing feature branch on the already-on-branch path.

## Investigate (calibrate to task size)

12. **Scale investigation to task size.**
    - **Size S**: Read the target, form a hypothesis, start coding. Max 2-3 queries.
    - **Size M**: Targeted investigation — trace call chains, check callers. Delegate bulk analysis to agents.
    - **Size L**: Deeper investigation, but prefer agents for data-heavy exploration.
    - **Rule**: Never run more than 2 sequential exploratory queries on the same question. If the second doesn't answer it, delegate or just try.

## Implement

13. Implement the backlog item. Read only the sections of files you need (`offset`/`limit`). Grep first to locate code.
14. After completing each sub-step, check it off immediately: `- [ ]` → `- [x]`.
15. Before writing test helpers, search for existing ones with the Grep tool (pattern `fn (ent|spawn_test|test_world)`, glob `crates/**/*.rs`, or type `rust`). Reuse — do NOT duplicate.
16. Write tests for any new or changed functionality. For each new test, verify: "Would this pass if the function returned a constant?" If yes, needs different inputs/assertions.
17. **Scaffolding dead code:** if clippy flags `dead_code` on items a *later* backlog task will consume, add `#[allow(dead_code)]` with a `// FIXME(T<n>):` comment naming the unblocking task. Never silence clippy without a FIXME.
18. **Smoke test before self-review.** If the task adds or changes a user-facing surface — a new `ether-forge` subcommand, a new CLI flag, a new agent/hook entry point, a file format — run it end-to-end against a realistic input *before* moving on. Unit tests verify code correctness in isolation; the smoke test verifies that the wiring, clap argv, error messages, and file-system side effects actually behave as documented. Typical shapes:
    - New subcommand: invoke it in a `mktemp -d` with happy-path args, one malformed-input case, and (if relevant) one stdin case. Eyeball the output.
    - New hook: source it from a scratch shell and confirm idempotence on a clean state.
    - New file format: run the producer, `cat` the result, and feed it back into the consumer.

    Catching wiring bugs here is much cheaper than discovering them after the reviewer agent has already started.

## Self-Review + Commit (atomic — do not split across turns)

> **CRITICAL:** Steps 19–22 form a single unbroken sequence. Do NOT end your turn between launching the reviewer and running `ether-forge commit`. The reviewer's background completion notification arrives in the same turn; wait for it, address findings, then commit. Ending the turn mid-sequence leaves uncommitted work that the stop-hook will flag.

19. **Size the review.** Run `git diff main --stat` and scan `git diff main` for `unsafe`, `HashMap`, or new test files.
    - If the diff is under **30 changed lines** AND has none of those markers: skip the subagent and self-review inline against the checks listed in `.claude/agents/reviewer.md` while `ether-forge check` runs. Then proceed directly to step 22.
    - Otherwise: launch both of the following **in a single message** (one foreground `Bash`, one background `Agent`):
      - **Background:** Spawn the reviewer subagent (`subagent_type: reviewer`, `run_in_background: true`). The agent is pinned to `haiku` and owns its own tool allowlist — do not override.
      - **Foreground:** `ether-forge check` (fmt + clippy + test in one call).

    Review subagent prompt (pass the worktree path and task ID only — the agent resolves its own context and fetches the diff itself, so neither the task body nor the diff ever enters the parent context):

    > Review task **T<n>** in worktree `/home/arthur/ether/.claude/worktrees/dev-T<n>`.
    >
    > `cd` into that worktree, read `CLAUDE.md` and `.claude/rules/*.md`, run `ether-forge task T<n> --context` for the goal, then `git diff main` for the change.
    >
    > Apply the checklist in your system prompt and return a terse findings list.

20. **Wait for the reviewer completion notification in the same turn.** Do NOT emit a "waiting for reviewer" message and end your turn — that's the failure mode the stop-hook catches. Continue with other non-blocking work (e.g. drafting the commit body) while the reviewer runs, but stay in the turn until the `<task-notification status="completed">` arrives.
21. **Address findings.** Re-run `ether-forge check` if anything changed. If the reviewer writes `target/.ether-forge/review-T<n>.json` with non-empty `blockers`, `ether-forge commit` will refuse the commit until they're resolved — pass `--force-review` only as a deliberate override (it stamps a `Reviewed-by-force: true` trailer). If the fix is non-trivial, loop back to step 19.
22. **Commit immediately** — same turn as the reviewer result. ether-forge runs `check` then invokes `git commit` with the task title as the subject:
    ```bash
    ether-forge commit T<n> -a
    ```
    Pass extra `-m` flags for a body explaining the *why*. If the pre-commit hook fails, fix, stage, and create a NEW commit (do not amend).

## Wrap Up

23. **While still on the task branch**, mark the task done and cascade dependencies:
    ```bash
    ether-forge done T<n> --commit $(git rev-parse --short HEAD)
    ```
    This moves the file to `backlog/done/`, strips sub-steps, and unblocks dependents. Commit the resulting backlog changes.
24. Report: branch name, what changed, `ether-forge check` result.
25. **Pre-merge hygiene:** on the fresh path, `ExitWorktree` with `action: "keep"` to return to main, then `git status` on main — if dirty, warn instead of merging. On the already-on-branch path, there is no worktree to exit; just run `git status` against main after you switch.
26. Use `AskUserQuestion` to ask whether to merge and delete the branch (options: "Merge and delete" / "Keep branch"). On confirmation, pick the matching primitive for the path:

    **Fresh path (`dev-T<n>` worktree):**
    ```bash
    ether-forge merge T<n>
    ```
    Collapses the ff-merge / `git worktree remove` / `git branch -d` dance into one primitive: verifies the worktree is clean, rebases onto main if it advanced, re-runs `check`, applies the reviewer-blocker gate, ff-merges, then removes the worktree directory and deletes the branch. Pass `--keep` to leave both in place, or `--force-review` to override a blocker artifact.

    **Already-on-branch path:** `ether-forge merge T<n>` also handles this case — when no linked `dev-T<n>` worktree exists, it falls back to an in-place merge of the primary worktree's currently-checked-out branch. The fallback verifies the tree is clean, rebases the branch onto main if it advanced, runs `check`, applies the reviewer-blocker gate, `git checkout main`s, ff-merges, then deletes the branch (honors `--keep`). Remote branch deletion and `git push origin main` are still your responsibility — run them after `ether-forge merge` returns.
