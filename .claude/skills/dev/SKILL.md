---
name: dev
description: Autonomous development session on the Ether ECS workspace. Picks a backlog item, implements it, tests it, self-reviews, and commits.
---

# Ether Development Session

Work autonomously on the Ether ECS workspace at `/home/arthur/ether`. Lean on `ether-forge` for every backlog operation — it handles parsing, worktree creation, verification, and dependency cascades.

## Setup

1. `cd /home/arthur/ether`
2. **Task selection:**
   - If the user passed a `T<n>` argument, use that ID.
   - Otherwise, `ether-forge next` prints the top ready task (priority, then ID).
   - If no ready tasks exist, stop: "No ready tasks. Run `/groom` to generate them."
3. **Stale detection + claim check:** `git branch --list 'T*'`. If a branch already exists for the picked ID, skip to the next ready task (`ether-forge list --status ready`). For other `T*` branches with no worktree, warn the user.
4. `ether-forge get T<n>` — inspect the task body. If any sub-steps are already checked (`- [x]`), this is a **resumed session**; report which are done and start from the first unchecked step.
5. If the task needs more context, read the relevant `ROADMAP.md` section.
6. `CLAUDE.md` is already in the system context — no need to re-read.
7. `ether-forge check` — verifies fmt/clippy/tests in one call. If it fails, fix the baseline first, commit the fix, then continue.

## Claim + Isolate

8. Create a worktree and branch via ether-forge (it derives the slug from the task title):
   ```bash
   ether-forge worktree T<n>
   cd worktrees/T<n>
   ```
9. All further work runs inside the worktree.

## Investigate (calibrate to task size)

10. **Scale investigation to task size.**
    - **Size S**: Read the target, form a hypothesis, start coding. Max 2-3 queries.
    - **Size M**: Targeted investigation — trace call chains, check callers. Delegate bulk analysis to agents.
    - **Size L**: Deeper investigation, but prefer agents for data-heavy exploration.
    - **Rule**: Never run more than 2 sequential exploratory queries on the same question. If the second doesn't answer it, delegate or just try.

## Implement

11. Implement the backlog item. Read only the sections of files you need (`offset`/`limit`). Grep first to locate code.
12. After completing each sub-step, check it off immediately: `- [ ]` → `- [x]`.
13. Before writing test helpers, search for existing ones (`grep -rn 'fn ent\|fn spawn_test\|fn test_world' crates/ tests/`). Reuse — do NOT duplicate.
14. Write tests for any new or changed functionality. For each new test, verify: "Would this pass if the function returned a constant?" If yes, needs different inputs/assertions.
15. **Scaffolding dead code:** if clippy flags `dead_code` on items a *later* backlog task will consume, add `#[allow(dead_code)]` with a `// FIXME(T<n>):` comment naming the unblocking task. Never silence clippy without a FIXME.

## Self-Review + Verify (parallel)

16. **Launch both in a single message:**
    - **Background:** Spawn a review subagent (`subagent_type: general-purpose`, `model: "haiku"`, `run_in_background: true`) with the diff and the task description.
    - **Foreground:** `ether-forge check` (fmt + clippy + test in one call).

Review subagent prompt:

> Review this diff for the Ether ECS Rust workspace. Check for:
>
> **Logic errors** — incorrect assumptions, flawed test logic, no-op tests, assertions that pass with constant returns
> **Duplication** — test helpers that already exist elsewhere in the workspace
> **Safety** — unsafe blocks without SAFETY comments, unsound abstractions
> **Determinism** — unsorted HashMap iteration, non-deterministic output
> **Whether the change matches the stated goal:** "<paste backlog item description>"
>
> The diff:
> <paste git diff output>

17. When both complete, address findings. Re-run `ether-forge check` if anything changed.

## Commit

18. Commit via ether-forge — it runs `check` then invokes `git commit` with the task title as the subject:
   ```bash
   ether-forge commit T<n> -a
   ```
   Pass extra `-m` flags for a body explaining the *why*. If the pre-commit hook fails, fix, stage, and create a NEW commit (do not amend).

## Wrap Up

19. **While still in the worktree**, mark the task done and cascade dependencies:
    ```bash
    ether-forge done T<n> --commit $(git rev-parse --short HEAD)
    ```
    This moves the file to `backlog/done/`, strips sub-steps, and unblocks dependents. Commit the resulting backlog changes.
20. Return to the main checkout and clean up:
    ```bash
    cd /home/arthur/ether
    git worktree remove worktrees/T<n>
    ```
21. Report: branch name, what changed, `ether-forge check` result.
22. **Pre-merge hygiene:** `git status` on main working tree. If dirty, warn instead of merging.
23. Use the `AskUserQuestion` tool to ask whether to merge and delete the branch (options: "Merge and delete" / "Keep branch"). On confirmation:
    ```bash
    git merge T<n>-<slug>
    git branch -d T<n>-<slug>
    ```
