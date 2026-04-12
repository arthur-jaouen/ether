---
name: close
description: Wrap up the current session — commit uncommitted changes (including worktrees) and report final state.
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
   d. If the worktree is a groom/roadmap branch, ask about merging

4. **Report final state:**
   ```
   ## Session closed

   ### Commits
   - <branch>: <short message> (<hash>)

   ### Active worktrees
   - <path> [<branch>] — clean | N uncommitted changes

   ### Backlog snapshot
   - N ready, N blocked, N done
   ```

## Rules

- Never force-push or amend existing commits
- Never commit `.env` or secrets
- If no changes anywhere, report "nothing to commit" + backlog snapshot
- If a worktree has half-finished work (failing tests), flag it instead of committing
- Don't push unless explicitly asked
