---
name: backlog
description: Day-to-day backlog management for Ether ECS. List, add, reorder, and complete tasks in backlog/ directory.
argument-hint: [action: list | add "task name" | done T<n> | reorder | status]
---

# Backlog

Manage the Ether task queue. Tasks live as individual files in `backlog/` with YAML frontmatter. Read the argument to determine the action — if none given, default to `status`.

## Setup

1. `cd /home/arthur/ether`
2. Scan `backlog/*.md` — each file has YAML frontmatter (`id`, `title`, `size`, `status`, `depends_on`, `priority`)

## Actions

### `list`

3. For each file in `backlog/*.md`, parse YAML frontmatter.
4. Check `git branch --list 'T*'` to detect in-progress tasks.
5. Render a table sorted by: priority (lowest first, no-priority last), then ID:

```
| ID   | Title                     | Size | Status  | Deps | Pri | Branch |
|------|---------------------------|------|---------|------|-----|--------|
| T1   | World and Entity types    | M    | ready   |      |     |        |
| T2   | Component storage         | M    | ready   |      |     |        |
```

6. Show counts: N ready, N draft, N blocked.

### `add "task name"`

3. Ask for sub-steps, or generate from context.
4. Auto-assign next ID: scan `backlog/` and `backlog/done/` for highest `T<n>`, increment.
5. Generate short slug (lowercase, hyphens, 2-4 words).
6. Create `backlog/T<next>-<slug>.md`:

```yaml
---
id: T<next>
title: <task name>
size: M
status: draft
---

## Sub-steps

- [ ] Sub-step 1
- [ ] Sub-step 2
```

7. Commit the new file.

### `done T<n>`

3. Find `backlog/T<n>-*.md`. If in `backlog/done/`, report already done.
4. Get implementation commit hash from git log or ask the user.
5. Create `backlog/done/T<n>-<slug>.md` (frontmatter only, status=done, commit field added, sub-steps stripped).
6. Delete original.
7. **Cascade:** scan remaining `backlog/*.md`. Remove `T<n>` from `depends_on` lists. Promote `blocked` → `ready` when list empty.
8. Report cascades. Commit all changes.

### `reorder`

3. Show current list sorted by priority then ID.
4. To bump: add/update `priority` field (lower = first).
5. To deprioritize: remove `priority` field.
6. Commit changes.

### `status`

3. Summary: count by status, next up (top ready), in progress (T* branches), blocked tasks, draft tasks.

## Rules

- New tasks always enter as `size: M`, `status: draft`. Run `/groom` to promote.
- Don't rewrite sub-step content — only manage lifecycle.
- Commit after every modification.
- File names: `T<n>-short-slug.md`.

## Paths

- Workspace: `/home/arthur/ether`
- Active: `/home/arthur/ether/backlog/`
- Done: `/home/arthur/ether/backlog/done/`
- Schema: `/home/arthur/ether/BACKLOG.md`
- Roadmap: `/home/arthur/ether/ROADMAP.md`
