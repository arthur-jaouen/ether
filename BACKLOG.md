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
- **Size**: S = 1-3 sub-steps / 1-2 files, M = 3-6 sub-steps / 2-4 files, L = 6+ sub-steps or new module
- **Status**: draft = needs refinement, ready = can be picked up, blocked = waiting on another task, done = completed
- **Dependencies**: `depends_on` list — only present when `blocked`.
- **Priority**: Optional integer — lower = picked first.
- **Commit**: Implementation commit hash — only present when `done`.
- **Sub-steps**: `- [ ]` / `- [x]` checkboxes. Checked off during implementation.
- **Done tasks**: Moved to `backlog/done/`, sub-steps stripped, `commit` field added.

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
