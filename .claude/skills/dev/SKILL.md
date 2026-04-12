---
name: dev
description: Autonomous development session on the Ether ECS workspace. Picks a backlog item, implements it, tests it, self-reviews, and commits.
---

# Ether Development Session

Work autonomously on the Ether ECS workspace at `/home/arthur/ether`.

## Setup

1. `cd /home/arthur/ether`
2. **Task selection:** Use `ls backlog/` (via Bash) to list task files. If the user passed a `T<n>` argument, find the matching file. Otherwise, Read all `backlog/*.md` files, parse YAML frontmatter to find `status: ready` tasks. Pick by: (1) `priority` field first (lower wins), (2) lowest `T<n>` ID as tiebreaker. If no `ready` tasks exist, stop: "No ready tasks. Run `/groom` to generate them."
3. **Stale detection + claim check:** Run `git branch --list 'T*'` — if a branch matches the picked task's ID, skip to the next `ready` task. For other `T*` branches with no worktree, warn the user.
4. If the task has checked sub-steps (`- [x]`), this is a **resumed session**. Report which are done and start from the first unchecked step.
5. If the task needs more context, check `ROADMAP.md`
6. Read `CLAUDE.md` — refresh architecture and anti-patterns
7. Run `cargo test --workspace` to verify a clean baseline
8. If baseline fails: fix it first, commit the fix, then continue

## Claim + Isolate

9. Create a worktree and branch atomically:
   ```bash
   git worktree add worktrees/T<n> -b T<n>-short-description main
   cd worktrees/T<n>
   ```
10. All work runs inside the worktree.

## Investigate (calibrate to task size)

11. **Scale investigation to task size.**
    - **Size S**: Read the target, form a hypothesis, start coding. Max 2-3 queries.
    - **Size M**: Targeted investigation — trace call chains, check callers. Delegate bulk analysis to agents.
    - **Size L**: Deeper investigation, but prefer agents for data-heavy exploration.
    - **General rule**: Never run more than 2 sequential exploratory queries on the same question. If the second doesn't answer it, delegate or just try.

## Implement

12. Implement the backlog item
    - Read only the sections of files you need (`offset`/`limit`). Use Grep to locate code first.
    - After completing each sub-step, immediately check it off: `- [ ]` → `- [x]`
13. Before writing test helpers, search for existing ones:
    - `grep -rn 'fn ent\|fn spawn_test\|fn test_world' crates/ tests/`
    - Reuse shared helpers — do NOT duplicate
14. Write tests for any new or changed functionality
    - For each new test, verify: "Would this pass if the function returned a constant?" If yes, needs different inputs/assertions.

## Self-Review + Verify (parallel)

15. **Launch both in a single message:**
    - **Background:** Spawn a review subagent (`model: "haiku"`, `run_in_background: true`) with the diff
    - **Foreground:** Run verification commands as parallel Bash calls:
      - `cargo test --workspace`
      - `cargo clippy --workspace -- -D warnings`
      - `cargo fmt --all -- --check`

16. Review subagent prompt (Agent tool, `subagent_type: general-purpose`, `model: "haiku"`, `run_in_background: true`):

> Review this diff for the Ether ECS Rust workspace. Check for:
>
> **Logic errors** — incorrect assumptions, flawed test logic, no-op tests, assertions that pass with constant returns
>
> **Duplication** — test helpers that already exist elsewhere in the workspace
>
> **Safety** — unsafe blocks without SAFETY comments, unsound abstractions
>
> **Determinism** — unsorted HashMap iteration, non-deterministic output
>
> **Whether the change matches the stated goal:** "<paste backlog item description>"
>
> The diff:
> <paste git diff output>

17. When both complete: address findings. Re-run only affected checks.

## Commit

18. Stage and commit with a descriptive message explaining the "why"
19. The pre-commit hook will run fmt, clippy, and tests
20. If the hook fails: fix, stage, create a NEW commit (do not amend)

## Wrap Up

21. **While still in the worktree**, finalize the task file:
    - Move `backlog/T<n>-*.md` to `backlog/done/T<n>-*.md` with `status: done` and sub-steps stripped
    - **Cascade:** scan all `backlog/*.md` for `depends_on` containing the completed ID. Remove it. If list empty → `ready`.
    - Commit the backlog changes
22. Return to main checkout and clean up:
    ```bash
    cd /home/arthur/ether
    git worktree remove worktrees/T<n>
    ```
23. Report: branch name, what changed, test results
24. **Pre-merge hygiene:** Run `git status` on main working tree. If dirty, warn instead of merging.
25. Ask if the user wants to merge and delete the branch:
    ```bash
    git merge T<n>-short-description
    git branch -d T<n>-short-description
    ```
