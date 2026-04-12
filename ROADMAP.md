# Ether ECS Roadmap

High-level priorities and context. Items here are ideas and goals — they move to `backlog/` when broken down into concrete tasks.

## Phase 0 — ether-forge (active focus)

Goal: a standalone CLI crate (`crates/ether-forge`) for managing the development process. Every skill (`/dev`, `/groom`, `/backlog`) shells out to it instead of parsing task files with ad-hoc bash loops.

### Backlog CLI

Binary: `cargo run -p ether-forge -- <subcommand>`. Subcommands:

- `list [--status ready|blocked|draft|done|all]` — tabular task list sorted by priority then ID
- `next` — print the top ready task (priority-first, lowest ID tiebreaker)
- `get T<n>` — show full task detail (frontmatter + sub-steps)
- `search <query>` — keyword search across task title, ID, and sub-step text
- `deps T<n>` — dependency tree (what it depends on + what depends on it)
- `status` — summary counts, next up, blocked list

Parses `backlog/*.md` and `backlog/done/*.md` YAML frontmatter. Standalone crate — no dependency on ether-core or ether-macros.

### Lifecycle subcommands (high leverage)

These eliminate the most error-prone manual steps in the dev loop:

- `done T<n> [--commit <sha>]` — mark task done, apply cascade rule (scan all tasks, remove completed ID from `depends_on`, flip `blocked` → `ready` when empty), move file to `backlog/done/`. Single atomic operation.
- `worktree T<n>` — create `worktrees/T<n>-<slug>` + branch `task/T<n>` from main, print the path to `cd` into.
- `commit T<n>` — run `check`, then `git commit` with a message prefixed by the task ID and title pulled from frontmatter.
- `check` — pre-commit verification: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`.
- `validate` — lint backlog integrity: orphan `depends_on`, duplicate IDs, malformed frontmatter, `blocked` tasks with empty deps, `done` tasks missing `commit` field.
- `groom` — audit logic currently embedded in the `/groom` skill (coverage diff vs ROADMAP, propose missing tasks).

### Git & CI automation

- `ether-forge install-hooks` — writes `.git/hooks/pre-commit` that runs `ether-forge check`. Opt-in, idempotent.

### Skill thinning

With the above in place, `/dev`, `/groom`, and `/backlog` skills become ~20-line orchestrators that shell out to `ether-forge` instead of parsing YAML in bash. Skills own conversation flow; ether-forge owns state mutation.

### Ordering

1. `done` + cascade (highest leverage — replaces the most fragile manual step)
2. `check` + `install-hooks` (unblocks safer autonomous commits)
3. `worktree` + `commit` (smooths the `/dev` loop)
4. `validate` (catches drift early)
5. `groom` migration (last, once the primitives are stable)

### Claude Code hooks

Configured in `.claude/settings.json`. The harness (not the model) runs these, so they're enforced guardrails and can't be talked out of. Hooks are independent of ether-forge — they shell out to whatever command is configured, so bash scripts can stand in until the CLI lands.

- **PostToolUse** (`Edit`, `Write` on `*.rs`) — run `cargo fmt` on the touched file. Sub-100ms, deterministic, keeps the tree formatted without waiting for commit. Clippy stays at commit time via `ether-forge check` — too slow for per-edit.
- **PreToolUse** (`Bash`) — block destructive patterns: `git push --force*`, `git reset --hard*`, `git branch -D *`, `rm -rf *`. User retains override by running the command themselves in a `!` prompt.
- **SessionStart** — inject backlog status into first context (next ready task, counts of ready/blocked/draft). Ship `.claude/hooks/backlog-status.sh` (pure bash over `backlog/*.md` frontmatter) now; swap the command to `ether-forge status` later with no hook-config change.
- **SessionEnd** — run `ether-forge validate` (or a bash equivalent in the interim). Fires once at session close, not per turn — silent during work, noisy only when drift is detected.

Rollout: hooks land alongside the matching ether-forge subcommands, but the bash fallbacks mean none of them are blocked on the Rust work.

## Phase 1 — Core ECS

Goal: a minimal but functional ECS with World, Entity, Component storage, and basic queries.

### World & Entity

The foundation. `World` owns all ECS state. `Entity` is a generational index — a `u32` index + `u32` generation. When an entity is despawned, its index is recycled but the generation increments, so stale `Entity` handles are detectable.

- `World::spawn() -> Entity` — allocate a new entity
- `World::despawn(entity)` — recycle the index, bump generation
- `World::is_alive(entity) -> bool` — check generation matches

### Component storage

Components are plain Rust structs implementing the `Component` trait. Each component type gets a unique `ComponentId` (derived from `TypeId`).

**Storage strategy**: Start with sparse sets (one per component type). A sparse set has:
- A sparse array indexed by entity index → dense index
- A dense array of `(Entity, ComponentData)` pairs
- O(1) insert/remove/lookup, cache-friendly iteration over the dense array

Future: archetype-based storage for batch iteration when the query system demands it.

- `World::insert<T: Component>(entity, component)` — add/replace component
- `World::remove<T: Component>(entity)` — remove component
- `World::get<T: Component>(entity) -> Option<&T>` — read component
- `World::get_mut<T: Component>(entity) -> Option<&mut T>` — write component

### Basic queries

Iterate over entities matching a component set:

```rust
// Read Position and Velocity for all entities that have both
for (pos, vel) in world.query::<(&Position, &Velocity)>() { ... }

// Mutable access
for (mut pos, vel) in world.query::<(&mut Position, &Velocity)>() { ... }
```

Filters come later (Phase 2).

## Phase 2 — Query filters & derive macros

### Query filters

- `With<T>` — entity must have component T (but don't fetch it)
- `Without<T>` — entity must not have component T
- `Option<&T>` — fetch if present, None otherwise

### Derive macros

- `#[derive(Component)]` — implement the `Component` trait
- `#[derive(Bundle)]` — a group of components that can be inserted together

## Phase 3 — Systems & scheduling

### System trait

A system is a function that borrows from the World:

```rust
fn movement_system(query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in &query {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### Scheduler

- Topological ordering based on declared dependencies
- Automatic parallelism: systems with non-overlapping borrows run concurrently
- Explicit ordering constraints: `system_a.before(system_b)`

## Phase 4 — Resources & events

### Resources

Global singletons stored in the World:

```rust
world.insert_resource(DeltaTime(0.016));
let dt = world.resource::<DeltaTime>();
```

### Events

Typed event channels with double-buffered reader pattern:

```rust
world.send_event(CollisionEvent { a, b });
for event in world.read_events::<CollisionEvent>() { ... }
```

## Phase 5 — Performance & polish

### Benchmarks

- Entity spawn/despawn throughput
- Component insert/remove throughput
- Query iteration (1-component, 3-component, 5-component)
- Fragmented iteration (many archetypes)

### Optimizations

- Archetype-based storage for query iteration
- Change detection (track which components were mutated)
- Parallel query iteration with `par_iter()`
