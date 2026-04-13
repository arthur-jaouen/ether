# Ether Backlog

Tasks live as individual files in `backlog/`. Completed tasks are in `backlog/done/`. See `ROADMAP.md` for context and priorities.

## Task file schema

Each file uses YAML frontmatter:

```yaml
---
id: T<n>
title: Short task description
size: S|M|L
status: draft|ready|blocked|done
depends_on:        # only when status is "blocked"
  - T<id>
priority: 1        # optional — lower number = picked first
commit: abc1234    # only when status is "done"
---

## Sub-steps

- [ ] Step 1
- [ ] Step 2
```

## Naming convention

File names include the task ID and a slug: `T1-world-entity.md`.

## Format rules

- **IDs**: `T<n>` — stable, auto-incremented integer. Never reused.
- **Size**: See the sizing rules below. Default to M when in doubt.
- **Status**: draft = needs refinement, ready = can be picked up, blocked = waiting on another task, done = completed
- **Dependencies**: `depends_on` list — only present when `blocked`.
- **Priority**: Optional integer — lower = picked first.
- **Commit**: Implementation commit hash — only present when `done`.
- **Sub-steps**: `- [ ]` / `- [x]` checkboxes. Checked off during implementation.
- **Done tasks**: Moved to `backlog/done/`, sub-steps stripped, `commit` field added.

## Sizing rules

| Size | Sub-steps | Files touched | When to pick it |
|------|-----------|---------------|-----------------|
| **S** | 1-3 | 1-2 | Self-contained change with no cross-crate reach. Examples: add a missing validation, rename a field, fix a bug with an accompanying regression test. |
| **M** | 3-6 | 2-4 | Default. New subcommand, new data structure, any change that touches both a producer and its consumers. |
| **L** | 6+ | 4+ or new module | Avoid if splittable. Only pick L when the change genuinely cannot be decomposed — a new crate, a trait refactor across the workspace, or a multi-phase migration. |

**Promote S → M when:**

- The change migrates a schema or on-disk format — every reader and writer must be updated in the same commit, so the blast radius is never truly 1-2 files.
- The change adds a new `ether-forge` subcommand — clap wiring, the cmd module, tests, and usually a doc/agent update land together.
- The change alters a public type in `ether-core` — the facade, macros, and any downstream crate drag in with it.
- Retrospective signal: if a shipped "S" ends up >~200 inserted lines or touches >3 files, write the follow-up groom note to bump the sizing heuristic rather than pretend it was S.

Under-sizing is an anti-pattern: it tempts you to cut test coverage or skip the self-review step to "stay within size S". Better to re-label mid-task and finish the work properly.

## Cascade rule

When a task completes:
1. Remove the completed ID from all other tasks' `depends_on` lists
2. If a task's `depends_on` list is now empty, remove it and change `status` from `blocked` to `ready`
3. If other IDs remain, keep `blocked` with the shortened list

## Quick reference

```bash
ls backlog/*.md          # list active tasks
ls backlog/done/*.md     # list completed tasks
```
