# Ether ECS

Rust workspace with three crates under `crates/`: a core ECS engine, derive macros, and a public facade.

## Building & Testing

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

Always run `cargo test --workspace` after changes.

## Architecture

All crates live under `crates/`.

**ether-core** — core ECS primitives (library):
- World, Entity (generational index), ComponentId
- Component storage (sparse sets, dense arrays)
- Query iteration, filters
- System trait, scheduling
- Resources (global singletons), Events

**ether-macros** — proc-macro crate:
- `#[derive(Component)]`, `#[derive(Bundle)]`
- Depends on `syn`, `quote`, `proc-macro2`

**ether** — public facade (re-exports core + macros):
- The crate users depend on
- Re-exports everything from `ether-core` and `ether-macros`

## Dependency graph

```
ether-core
  (standalone)
    ^
ether-macros       
  (syn, quote)
    ^
ether
  (ether-core, ether-macros)
```

## Key patterns

- **Generational entities**: `Entity` = index + generation. Reuse indices after despawn, generation prevents stale references.
- **Sparse sets**: Map `Entity` → component data. O(1) lookup, cache-friendly iteration when dense.
- **Archetypes** (future): Group entities by component set for batch iteration.
- **Determinism**: All iteration and output must be deterministic. Sort before iterating collections with non-deterministic order.

## Contributing (autonomous)

Before committing any change, verify ALL of these:

1. `cargo test --workspace` — all tests pass
2. `cargo clippy --workspace -- -D warnings` — zero warnings
3. `cargo fmt --all -- --check` — properly formatted
4. If you added a public type/function: add a rustdoc comment
5. If you changed ether-core public API: verify ether facade still compiles
6. Update the task file in `backlog/` to check off completed sub-steps

### Test expectations

- New data structures: unit tests with insert/remove/iterate operations
- New queries: tests with multi-component setups
- Proc macros: compile-pass and compile-fail tests
- Bug fixes: regression test proving the fix
- Unsafe code: tests exercising the boundary conditions

### Anti-patterns (lessons learned)

**Test fixture duplication** — Before creating test helpers, search the workspace for existing ones. Add new helpers to shared test modules, not inline.

**Non-deterministic iteration** — Never depend on HashMap/HashSet iteration order in output or tests. Always sort or use BTreeMap.

**Premature unsafe** — Try safe Rust first. Only use unsafe when benchmarks prove it's necessary, and document the safety invariant.

## Backlog maintenance

Tasks live as individual files in `backlog/` (YAML frontmatter + markdown body). `ROADMAP.md` has strategic context. `BACKLOG.md` documents the schema.

### Pipeline

| Step | Skill | What it does |
|------|-------|-------------|
| 1 | `/roadmap` | Update strategic direction |
| 2 | `/groom` | Audit coverage, lint integrity, generate missing tasks |
| 3 | `/dev` | Pick top `[ready]` item, implement, test, commit |

Day-to-day: `/backlog` for CRUD (list, add, done, reorder, status).

### Task file format

Each task is `backlog/T<n>-short-slug.md` with YAML frontmatter:

```yaml
---
id: T<n>
title: Short task description
size: S|M|L
status: draft|ready|blocked|done
depends_on:        # only when status is "blocked"
  - T<id>
priority: 1        # optional — lower = picked first
commit: abc1234    # only when status is "done"
---
```

### Size definitions

| Size | Sub-steps | Files touched | Guidance |
|------|-----------|---------------|----------|
| **S** | 1-3 | 1-2 | Single focused change |
| **M** | 3-6 | 2-4 | Default for most tasks |
| **L** | 6+ | 4+ or new module | Avoid if splittable |

### Cascade rule

When a task completes, scan all others for `depends_on` containing the completed ID:
1. Remove the completed ID from their list
2. If list now empty, remove `depends_on` and change `blocked` → `ready`
3. If other IDs remain, keep `blocked`

### Paths

- Workspace: `/home/arthur/ether`
- Backlog: `/home/arthur/ether/backlog/`
- Done: `/home/arthur/ether/backlog/done/`
- Roadmap: `/home/arthur/ether/ROADMAP.md`
- Schema: `/home/arthur/ether/BACKLOG.md`
