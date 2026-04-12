---
name: backlog
description: Day-to-day backlog management for Ether ECS. List, add, reorder, and complete tasks in backlog/ directory.
argument-hint: [action: list | next | get T<n> | search <query> | deps T<n> | add "task name" | done T<n> | reorder | status]
---

# Backlog

Manage the Ether task queue via the `ether-forge` CLI. Tasks live as individual files in `backlog/` with YAML frontmatter. Parse the argument to pick an action — if none given, default to `status`.

All commands run from `/home/arthur/ether`.

## Actions

### `list`

Shell out — `ether-forge` already sorts by priority then ID and renders a table:

```bash
ether-forge list
```

Optionally filter: `ether-forge list --status ready`.

### `status`

```bash
ether-forge status
```

Returns counts by status and the next ready task. Done.

### `next`

```bash
ether-forge next
```

Prints the top ready task (priority ascending, then numeric id). Use when the user asks "what's next?" without specifying an id.

### `get T<n>`

```bash
ether-forge get T33
```

Prints the full task body (frontmatter + markdown) for one id. Searches active and `done/` — use when the user references a task by id and you need its sub-steps or body.

### `search <query>`

```bash
ether-forge search worktree
```

Case-insensitive substring match across id, title, and body. Use when the user describes a topic without an id ("is there already a task about X?") to avoid duplicate adds.

### `deps T<n>`

```bash
ether-forge deps T33
```

Shows upward (what T33 blocks on) and downward (what depends on T33) edges. Use before marking `done` or reprioritizing to see the cascade impact.

### `add "task name"`

`ether-forge` does not generate task bodies — this action stays conversational:

1. Ask the user (or infer from context) for sub-steps.
2. Pick the next ID: `ether-forge list` shows the highest active ID; also scan `backlog/done/` for higher.
3. Generate a short slug (lowercase, hyphens, 2-4 words).
4. Write `backlog/T<next>-<slug>.md` with frontmatter (`id`, `title`, `size: M`, `status: draft`) and a `## Sub-steps` section.
5. `ether-forge validate` to confirm integrity.
6. Commit the new file.

### `done T<n>`

```bash
ether-forge done T<n> --commit <sha>
```

Moves the file to `backlog/done/`, strips sub-steps, records the commit, and cascades `depends_on` updates across the backlog. Commit the resulting changes. If the user does not supply a sha, read it from `git log` for the implementing commit.

### `reorder`

Priority lives in the `priority` frontmatter field (lower = first). `ether-forge` has no reorder subcommand yet, so:

1. `ether-forge list` to show the current order.
2. Edit the `priority` field on the tasks the user wants to bump (or remove it to deprioritize).
3. `ether-forge validate` and commit.

## Rules

- New tasks always enter as `size: M`, `status: draft`. Run `/groom` to promote.
- Don't rewrite sub-step content — only manage lifecycle.
- Commit after every modification.
- File names: `T<n>-short-slug.md`.
- Prefer shelling out to `ether-forge` over parsing frontmatter by hand. Fall back to direct file edits only when no subcommand covers the action.

## Paths

- Workspace: `/home/arthur/ether`
- Active: `/home/arthur/ether/backlog/`
- Done: `/home/arthur/ether/backlog/done/`
- Schema: `/home/arthur/ether/BACKLOG.md`
- Roadmap: `/home/arthur/ether/ROADMAP.md`
