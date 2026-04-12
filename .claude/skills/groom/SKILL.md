---
name: groom
description: Audit, lint, and generate backlog tasks in one pass — diffs ROADMAP vs backlog, validates integrity, proposes new tasks and fixes, applies on confirmation.
argument-hint: ["all" (default) | section name (e.g. "Phase 2") | task ID (e.g. T4)]
---

# Groom

Unified backlog health check: audit coverage, lint integrity, generate missing tasks. Shell out to `ether-forge groom` for the mechanical work; reserve conversation for judgment calls.

All commands run from `/home/arthur/ether`.

## Setup

1. `ether-forge groom --json` — dry-run report as JSON. Covers coverage diff vs `ROADMAP.md`, structural lint (IDs, `depends_on`, cycles, filenames, status consistency), and proposed cascade fix-ups.
2. Parse argument for scope:
   - no argument → full groom
   - section name → filter report to that roadmap section
   - task ID (`T<n>`) → skip coverage/generation; just surface lint findings for that task (`ether-forge get T<n>` + the relevant section of the JSON report).

## Phase 1 — Read the report

3. Inspect the JSON findings from `ether-forge groom`:
   - **Coverage:** each roadmap section classified Covered / Partial / Uncovered / Done.
   - **Lint:** duplicate IDs, dangling `depends_on`, cycles, status/`depends_on` mismatches, filename drift.
   - **Cascade candidates:** blocked tasks whose deps are all done (safe auto-fix).
4. For single-task-ID scope, stop after lint findings for that task.

## Phase 2 — Sub-step grounding (scoped)

5. Only for tasks being modified, flagged stale, or newly generated, spot-check sub-step references:
   - File references: Glob for each path. Flag missing.
   - Function/type references: Grep for each. Flag missing.
6. Count sub-steps vs `size` tag; flag mismatches.

## Phase 3 — Generate (for uncovered sections)

*Skip for single task ID scope.*

7. For each uncovered or partially covered roadmap section:
   a. Read the section — extract deliverables and dependencies.
   b. Explore the codebase to ground deliverables (Grep/Glob for files, types, functions).
   c. Size the work (S/M/L). Prefer tasks completable in a single `/dev` session.
   d. Assign IDs starting from `T<max+1>` (the JSON report includes the current max).
   e. Sub-steps start with a verb and reference specific file paths.
   f. Wire dependencies.
   g. If a section lacks detail, flag it rather than producing vague tasks.

## Phase 4 — Propose

8. Collect findings into three buckets:
   - **Auto-applied** (always safe): cascade satisfied deps — `ether-forge groom --apply` handles this.
   - **Proposed** (need "yes"): new tasks, size corrections, status fixes, ROADMAP.md updates.
   - **Flagged** (info only): circular deps, orphaned tasks.
9. Present a concise report and wait for confirmation.

## Phase 5 — Apply

10. Create a groom worktree:
    ```bash
    git worktree add worktrees/groom-YYYY-MM-DD -b groom-YYYY-MM-DD main
    cd worktrees/groom-YYYY-MM-DD
    ```
11. Apply auto-fixes via `ether-forge groom --apply`. Apply proposed changes (new task files, edits, ROADMAP.md updates) directly in the worktree.
12. `ether-forge validate` to confirm integrity before committing.
13. Commit with a descriptive message.
14. Ask whether to merge into `main` and clean up the worktree.

## Rules

- `ether-forge groom` is the source of truth for coverage + lint mechanics. Don't re-implement parsing in the skill.
- ROADMAP.md changes are proposed like any other change — include them in the report and apply on confirmation.
- Auto-fix only safe operations (cascade). Everything else needs confirmation.
- New tasks go to `ready` or `blocked` — never `draft`.
- Collect all findings before reporting — don't ask after each.
- Don't rewrite task content during lint — validate structure, not substance.
