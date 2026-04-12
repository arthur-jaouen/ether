---
name: groom
description: Audit, lint, and generate backlog tasks in one pass — diffs ROADMAP vs backlog, validates integrity, proposes new tasks and fixes, applies on confirmation.
argument-hint: ["all" (default) | section name (e.g. "Phase 2") | task ID (e.g. T4)]
---

# Groom

Unified backlog health check: audit coverage, lint integrity, generate missing tasks. Batch-and-confirm.

## Setup

1. `cd /home/arthur/ether`
2. Read `ROADMAP.md` (CLAUDE.md is already in system context).
3. Read all task frontmatter in one bash call:
   ```bash
   for f in backlog/*.md backlog/done/*.md; do echo "==$f=="; head -10 "$f"; done
   ```
4. Parse argument for scope: no argument = full groom, section name = scoped, task ID = lint only.

## Phase 1 — Audit (coverage analysis)

*Skip for single task ID scope.*

5. **Map roadmap sections to task files.** Each section should have corresponding tasks.
6. **Classify:** Covered / Partially covered / Uncovered / Done.
7. **Detect stale tasks**: sub-steps reference files/functions that no longer exist.
8. **Detect orphaned tasks**: don't trace to any roadmap section.

## Phase 2 — Lint (structural integrity)

### ID integrity
9. Every `T<n>` ID exactly once across `backlog/` and `backlog/done/`. No reuse.

### Dependency integrity
10. Every ID in `depends_on` must exist. No self-dependency. No circular deps.
11. Satisfied deps (all in done/) → remove `depends_on`, change `blocked` → `ready`.

### Status consistency
12. `blocked` requires `depends_on`. `depends_on` requires `blocked`. `ready` must have no unsatisfied deps.

### Sub-step validation (scoped)
13. Only for tasks being modified, flagged stale, or newly generated:
    - File references: Glob for each path. Flag missing.
    - Function/type references: Grep for each. Flag missing.

### Size validation
14. Count sub-steps vs size tag. Flag mismatches.

### Structural checks
15. Every active task needs `id`, `title`, `size`, `status`. File name matches `T<n>-<slug>.md`.

## Phase 3 — Generate (for uncovered sections)

*Skip for single task ID scope.*

16. For each uncovered/partially covered roadmap section:
    a. Read the section — extract deliverables and dependencies
    b. Explore codebase to ground deliverables (Grep/Glob for files, types, functions)
    c. Size the work (S/M/L)
    d. Split into tasks completable in a single `/dev` session
    e. Assign IDs from `T<max+1>`
    f. Sub-steps: each starts with a verb, references specific file paths
    g. Wire dependencies
    h. If section lacks detail, flag it rather than producing vague tasks

## Phase 4 — Propose

17. Collect all findings:

**Auto-applied** (always safe): cascade satisfied deps.

**Proposed changes** (need "yes"): new tasks, size corrections, status fixes.

**Flagged** (info only): stale refs, circular deps, orphaned tasks.

18. Present report. Wait for confirmation.

## Phase 5 — Apply

19. Create a worktree:
    ```bash
    git worktree add worktrees/groom-YYYY-MM-DD -b groom-YYYY-MM-DD main
    ```
20. Apply changes in the worktree.
21. Commit with descriptive message.
22. Ask user if they want to merge into `main` and clean up.

## Rules

- Do NOT modify ROADMAP.md — flag stale text, recommend `/roadmap`.
- Auto-fix only safe operations. Everything else needs confirmation.
- New tasks go to `ready` or `blocked` — never `draft`.
- Collect all findings before reporting — don't ask after each.
- Don't rewrite task content during lint — validate structure, not substance.
