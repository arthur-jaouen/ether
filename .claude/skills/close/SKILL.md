---
name: close
description: Wrap up the current session — commit uncommitted changes (including worktrees) and report final state.
argument-hint: []
---

# Close session

Commit outstanding changes and report final state.

## Steps

1. `cd /home/arthur/ether`

2. **Detect active worktrees:**
   ```bash
   git worktree list
   ```
   For each worktree (besides main), check for uncommitted changes.

3. **For each location with changes** (main + worktrees):
   a. Run `git status` and `git diff --stat`
   b. Stage relevant files (prefer specific files over `git add -A`)
   c. Commit with descriptive message ending with:
      ```
      Co-Authored-By: Claude <noreply@anthropic.com>
      ```
   d. For any non-main branch with commits ahead of `main` (dev-T<n>, groom-*, roadmap-*), use `AskUserQuestion` to ask about merging (options: "Merge and delete" / "Keep branch"). Dev worktrees mid-task (unchecked sub-steps) should be flagged as "Keep", not offered for merge.

4. **Report final state.** Run `ether-forge status` and paste its two lines verbatim under **Backlog snapshot** — don't hand-count or paraphrase. Example shape:
   ```
   ## Session closed

   ### Commits
   - <branch>: <short message> (<hash>)

   ### Active worktrees
   - <path> [<branch>] — clean | N uncommitted changes

   ### Backlog snapshot
   backlog: 9 tasks — 6 ready, 3 blocked, 0 draft, 0 done
   next: T<n>  <title of next ready task>
   ```

## Rules

- Never force-push or amend existing commits
- Never commit `.env` or secrets
- If no changes anywhere, report "nothing to commit" + backlog snapshot
- If a worktree has half-finished work (failing tests), flag it instead of committing
- Don't push unless explicitly asked
