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

8. `ether-forge preflight --task T<n>` — refuses if `main` is dirty, the current branch is behind `main`, or a branch already claims the id. Fix whatever it reports before going further. Skip this step if the session is already inside a worktree (preflight is for the *pre-entry* environment).
9. Call `EnterWorktree` with `name: "dev-T<n>"` so every tool (Glob/Grep/Read/Edit/Bash) resolves against the isolated worktree. Skip this step if the session is already inside a worktree — `EnterWorktree` refuses to nest, so work in place on the current branch.
10. All further work runs inside the worktree.

## Investigate (calibrate to task size)

11. **Scale investigation to task size.**
    - **Size S**: Read the target, form a hypothesis, start coding. Max 2-3 queries.
    - **Size M**: Targeted investigation — trace call chains, check callers. Delegate bulk analysis to agents.
    - **Size L**: Deeper investigation, but prefer agents for data-heavy exploration.
    - **Rule**: Never run more than 2 sequential exploratory queries on the same question. If the second doesn't answer it, delegate or just try.

## Implement

12. Implement the backlog item. Read only the sections of files you need (`offset`/`limit`). Grep first to locate code.
13. After completing each sub-step, check it off immediately: `- [ ]` → `- [x]`.
14. Before writing test helpers, search for existing ones (`grep -rn 'fn ent\|fn spawn_test\|fn test_world' crates/ tests/`). Reuse — do NOT duplicate.
15. Write tests for any new or changed functionality. For each new test, verify: "Would this pass if the function returned a constant?" If yes, needs different inputs/assertions.
16. **Scaffolding dead code:** if clippy flags `dead_code` on items a *later* backlog task will consume, add `#[allow(dead_code)]` with a `// FIXME(T<n>):` comment naming the unblocking task. Never silence clippy without a FIXME.

## Self-Review + Verify (parallel)

17. **Size the review.** Run `git diff main --stat` and scan `git diff main` for `unsafe`, `HashMap`, or new test files. If the diff is under **30 changed lines** AND has none of those markers, skip the subagent and self-review inline against the checks listed in `.claude/agents/reviewer.md` while `ether-forge check` runs. Otherwise, launch both of the following in a single message:
    - **Background:** Spawn the reviewer subagent (`subagent_type: reviewer`, `run_in_background: true`). The agent is pinned to `haiku` and owns its own tool allowlist — do not override.
    - **Foreground:** `ether-forge check` (fmt + clippy + test in one call).

Review subagent prompt (pass the worktree path and task ID only — the agent resolves its own context and fetches the diff itself, so neither the task body nor the diff ever enters the parent context):

> Review task **T<n>** in worktree `/home/arthur/ether/.claude/worktrees/dev-T<n>`.
>
> `cd` into that worktree, read `CLAUDE.md` and `.claude/rules/*.md`, run `ether-forge task T<n> --context` for the goal, then `git diff main` for the change.
>
> Apply the checklist in your system prompt and return a terse findings list.

18. When both complete, address findings. Re-run `ether-forge check` if anything changed. If the reviewer writes `target/.ether-forge/review-T<n>.json` with non-empty `blockers`, `ether-forge commit` will refuse the commit until they're resolved — pass `--force-review` only as a deliberate override (it stamps a `Reviewed-by-force: true` trailer).

## Commit

19. Commit via ether-forge — it runs `check` then invokes `git commit` with the task title as the subject:
   ```bash
   ether-forge commit T<n> -a
   ```
   Pass extra `-m` flags for a body explaining the *why*. If the pre-commit hook fails, fix, stage, and create a NEW commit (do not amend).

## Wrap Up

20. **While still in the worktree**, mark the task done and cascade dependencies:
    ```bash
    ether-forge done T<n> --commit $(git rev-parse --short HEAD)
    ```
    This moves the file to `backlog/done/`, strips sub-steps, and unblocks dependents. Commit the resulting backlog changes.
21. Report: branch name, what changed, `ether-forge check` result.
22. **Pre-merge hygiene:** before exiting the worktree, confirm the session is still inside it. `ExitWorktree` with `action: "keep"` to return to the main checkout, then `git status` on main. If dirty, warn instead of merging.
23. Use the `AskUserQuestion` tool to ask whether to merge and delete the branch (options: "Merge and delete" / "Keep branch"). On confirmation:
    ```bash
    git merge --ff-only dev-T<n>
    git worktree remove .claude/worktrees/dev-T<n>
    git branch -d dev-T<n>
    ```
    Prefer the explicit `git worktree remove` + `git branch -d` pair over re-entering just to call `ExitWorktree action: "remove"` — `ExitWorktree` is scoped to the active session only.
