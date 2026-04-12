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

### Worktree-native skills

Problem: `/groom`, `/dev`, and `/roadmap` currently create worktrees with `git worktree add … && cd …`. That only moves the Bash tool's cwd — Glob/Grep/Read/Edit keep resolving against the original session directory, so edits and lint re-runs silently drift back to `main`. Parallel runs of these skills step on each other for the same reason.

Fix: use the harness primitive `EnterWorktree` / `ExitWorktree` instead. `EnterWorktree` switches the whole session's working directory, so every tool — Bash, Glob, Grep, Read, Edit, and `ether-forge` invocations — naturally targets the worktree with no per-call path discipline.

Scope:

- Migrate `/groom`, `/dev`, `/roadmap` skill files to call `EnterWorktree` (named `groom-YYYY-MM-DD`, `dev-T<n>`, `roadmap-<topic>`) in place of the manual `git worktree add` + `cd` dance. Commit inside, then `ExitWorktree` with `keep` (pre-merge) or `remove` (post-merge) based on user choice.
- Document the nesting edge case: `EnterWorktree` refuses when already inside a worktree. Each skill notes "if already in a worktree, work in place" as a one-line fallback.
- Follow-up (independent, smaller): teach `ether-forge` to resolve repo root via `git rev-parse --show-toplevel` so subcommands stay correct when invoked from a subdirectory. No longer load-bearing for the worktree bug, but keeps the CLI robust on its own.

Non-goal: any change to `ether-forge`'s `--backlog` flag semantics. Inside an `EnterWorktree` session, cwd *is* the worktree and the existing relative default is already correct.

Ordering: skill migrations land first (S each, independent). The forge `--show-toplevel` resolver is a separate S task with no dependency on the skill work.

## Phase 0.5 — Claude tooling

Goal: make Claude's exploration, edit, and feedback loop cheaper and more semantically accurate. Where Phase 0 automates the *process*, this phase automates the *code work itself*. Shipped as `ether-forge` subcommands where it fits, standalone tools where it doesn't.

Ordering is deliberate: each phase is independently useful, and each later phase is a larger commitment than the last. Stop after 0.5.2 if 0.5.3 stops looking worth it.

### 0.5.1 — Feedback loop (highest leverage, smallest scope)

Rewrite `ether-forge check` to minimize wall-clock and output tokens. On a green run it should print ~2 lines; on failure it should print only errors, grouped, no progress noise.

Concrete invocation:

```
CARGO_TERM_COLOR=never cargo clippy --workspace --all-targets \
  --message-format=short -q -- -D warnings \
 && CARGO_TERM_COLOR=never cargo nextest run --workspace \
  --failure-output=final --status-level=fail --hide-progress-bar
```

- Clippy subsumes `cargo check` (runs it internally) — one pass instead of two.
- `--message-format=short` gives `file:line: error: msg` — ~10× cheaper in tokens than human output, no JSON parsing needed.
- `cargo-nextest` replaces `cargo test`: per-test process isolation, ~30% faster, grouped failure output. One-time `cargo install cargo-nextest`. Doctests still need `cargo test --doc` separately — add as a third step or accept the gap.
- Fail-fast via `&&` — no point running tests on a broken build.

Non-goals: sccache (marginal at 3 crates, can slow proc-macros), experimental `--changed-since` test impact (file-level only, `-p <crate>` is nearly as good).

### 0.5.2 — Structural search & refactor

Wrap [ast-grep](https://ast-grep.github.io/) as `ether-forge find` and `ether-forge rewrite` (or standalone — ast-grep is already a good CLI). The point isn't to reinvent it; it's to standardize *one* structural tool so skills and Claude converge on it instead of falling back to regex.

Capabilities gained:

- Find patterns like `$X.unwrap()`, `HashMap::new()`, `match $E { $$$ARMS }` with Rust-aware parsing.
- Rewrite field renames across struct literals *and* patterns in one pass (regex can't do this safely).
- Rule files checked into `.claude/rules/sg/` for repeatable audits (e.g. "no `.unwrap()` in ether-core").

Fall back to Grep when: single-identifier rename in a tight scope, doc/comment edits, or non-Rust files (`backlog/*.md`). Fall back to `rust-analyzer ssr` only for workspace-wide public-API renames where import resolution matters.

### 0.5.3 — Semantic navigation (largest, highest risk — gated)

`ether-forge nav callers <sym>`, `nav impls <trait>`, `nav def <sym>`, `nav type <file>:<line>:<col>`. Backed by **rust-analyzer as a persistent daemon** — the only option on stable Rust that answers all four queries correctly (rustdoc JSON is nightly and can't see call sites; syn can't do type resolution; rustc metadata has no stable consumer API).

Architecture:

- `ether-forge nav` starts a long-lived `rust-analyzer` subprocess on first use, speaks LSP over stdio, caches the pid in `target/.ether-forge/` or similar.
- `workspace/symbol` resolves `foo::bar` → position before issuing the real query, so the CLI surface stays name-based.
- Cold start is 5-30s on this workspace; warm queries are ms. Daemon idle-timeout (~5 min) prevents zombie processes.

**Gate:** don't start this until 0.5.1 and 0.5.2 are in use and their limits are felt. At 3 crates, grep + ast-grep may already cover 90% of navigation needs. Revisit when the codebase grows or when a concrete refactor is blocked by "find all callers."

### 0.5.4 — Review agent efficiency

Goal: shrink the self-review step in `/dev` (currently `.claude/skills/dev/SKILL.md:50-75`). Today it spawns a `general-purpose` haiku subagent with the entire diff pasted inline — the diff lives in both parent and child context, the agent's toolset is wider than needed, and it always fires regardless of diff size.

**Pass A — quick wins (no new tooling):**

- **Don't paste the diff.** The subagent `cd`s into the worktree and runs `git diff main` itself. Parent context stays clean; the review payload never enters the main conversation.
- **Add a custom `reviewer` agent** at `.claude/agents/reviewer.md`. Pinned `model: haiku`, minimal tool allowlist (`Read`, `Grep`, `Glob`, `Bash(git diff:*)`), terse system prompt that instructs it to read `CLAUDE.md` + `.claude/rules/*.md` at invocation so rules stay single-sourced. `/dev` just spawns `subagent_type: reviewer` with the task ID — no inline checklist. Reusable by `/groom` and future `/review` commands.
- **Skip on trivial diffs.** If `git diff main --stat` shows <30 changed lines and no `unsafe` / `HashMap` / new test files, self-review inline in the main loop instead of spawning at all.

**Pass B — forge-backed review toolkit:**

The review agent stops driving raw Grep/Read and instead composes `ether-forge` primitives. Forge exposes building blocks; the agent still reasons and decides what to flag.

- `ether-forge diff` — scoped worktree diff, lockfiles stripped, size-capped, ready to feed a reviewer
- `ether-forge task <T<n>> --context` — frontmatter + body + linked ROADMAP section as one blob, so the agent learns the goal without reading three files
- `ether-forge grep <recipe>` — shared recipes (`unsafe-without-safety`, `hashmap-iter`, `todo`, `dead-code`) so every agent uses the same patterns
- `ether-forge helpers` — registry of known shared test helpers, for duplication checks by name

Other skills (`/groom`, future review-only slash commands) reuse the same primitives, so the investment compounds.

**Ordering:** Pass A first — it's a skill-file edit, no Rust work, and it removes the biggest cost (inline diff) immediately. Pass B lands as small forge subcommands one at a time, each independently useful, and the skill migrates to them incrementally.

### Ordering

1. 0.5.1 feedback loop (S, `ether-forge check` rewrite + nextest install docs)
2. 0.5.2 ast-grep wrapper (S or M, depending on whether it's a thin pass-through or adds a rule-file convention)
3. 0.5.4 Pass A — review agent quick wins (S, skill-file edit only)
4. 0.5.4 Pass B — forge review primitives (M, one subcommand at a time)
5. 0.5.3 rust-analyzer daemon (L, gated — revisit decision before starting)

## Phase 0.5.5 — Review loop closure

Goal: finish the review-agent investment. Pass B shipped four forge primitives (`diff`, `task --context`, `grep <recipe>`, `helpers`) to back the reviewer, but the reviewer agent's tool allowlist never granted `ether-forge`, so the primitives are unused. Close that gap, plug the known doctest hole in `check`, and make `blocker` findings actually block commits.

### 0.5.5.1 — Wire reviewer to Pass B (S)

`.claude/agents/reviewer.md`:

- Extend `tools:` with `Bash(ether-forge diff:*)`, `Bash(ether-forge task:*)`, `Bash(ether-forge grep:*)`, `Bash(ether-forge helpers:*)`.
- Rewrite "On every invocation" to lead with forge primitives:
  1. `ether-forge task <id> --context` — goal + linked ROADMAP in one blob
  2. `ether-forge diff` — scoped, lock-stripped, size-capped diff (replaces raw `git diff main`)
  3. `ether-forge grep unsafe-without-safety | hashmap-iter | todo | dead-code` — systematic recipe sweep
  4. `ether-forge helpers` before flagging any "duplicate helper" finding
- Update `/dev` step 16 prompt to pass only `task_id` and worktree path — the agent resolves context itself, shrinking parent context further.

### 0.5.5.2 — Close doctest gap in `ether-forge check` (S)

`ether-forge check` currently runs clippy + nextest. Doctests are silently skipped — a known blind spot acknowledged in 0.5.1 and never resolved.

- Add a third chained step: `cargo test --workspace --doc -q` (fail-fast after nextest).
- Accept the ~1–2s cost; still a single invocation from the skill's POV.
- Regression test: a doctest with `assert!(false)` must make `ether-forge check` exit nonzero.

### 0.5.5.3 — Enforce reviewer severity (M)

Today `/dev` step 17 says "address findings" — the main agent decides, and `blocker` items carry no mechanical weight. Make the enforcement structural.

- **Reviewer output contract.** `.claude/agents/reviewer.md` emits a JSON artifact to a well-known path (`target/.ether-forge/review-T<n>.json`) with shape `{"blockers": [...], "nits": [...]}`, alongside the terse prose summary for the main agent.
- **Forge commit gate.** `ether-forge commit` auto-discovers the JSON artifact for the current task ID. If `blockers` is non-empty, the command refuses with a listing. `--force-review` escape hatch for documented false positives, recorded in a commit trailer.
- **Skill wiring.** `/dev` step 16 points the reviewer at the well-known path; step 18's existing `ether-forge commit T<n> -a` picks it up with no extra orchestration.

### Ordering

1. 0.5.5.1 reviewer wiring (S) — unblocks dormant Pass B investment immediately; pure config/prompt edit.
2. 0.5.5.2 doctest gap (S) — independent, small, removes a known blind spot.
3. 0.5.5.3a reviewer JSON output contract (S) — depends on 0.5.5.1.
4. 0.5.5.3b forge commit gate + /dev wiring (S) — depends on 0.5.5.3a.

Non-goals: reviewer model change (stays on haiku), telemetry/velocity tracking, parallel-`/dev` coordination.

## Phase 0.5.6 — Analysis skill grounding

Goal: the analysis-oriented skills (`/roadmap`, `/close`, `/brainstorm`) currently call zero `ether-forge` subcommands. They re-read files and reason from stale context instead of querying the source of truth that `/dev` and `/groom` already rely on. Close that gap so every skill treats `ether-forge` as a shared query layer, not a `/dev`+`/groom` private tool.

Observed gaps from a pass over `.claude/skills/*/SKILL.md`:

- **`/roadmap`** opens without grounding in backlog state. Should start with `ether-forge status` + `ether-forge list` so strategic edits reflect what's actually ready/blocked, and optionally `ether-forge groom --json` (dry-run) to see coverage drift vs this file before editing it.
- **`/close`** never summarises backlog deltas in its wrap-up. A single `ether-forge status` call at the end gives "X ready, Y blocked, Z done this session" for free.
- **`/brainstorm`** never checks whether an idea already exists. `ether-forge search <keyword>` and `ether-forge deps T<n>` prevent duplicate suggestions and surface blockers on adjacent work.
- **`/backlog`** uses `list`, `status`, `validate`, `done` but not `get`, `search`, `deps`, or `next` — all natural day-to-day CRUD ops currently re-implemented in prose.

Cleanup: remove the `ether-forge worktree T<n>` subcommand. It's dead code in every agent-driven skill — `EnterWorktree` is the only primitive that re-roots Glob/Grep/Read/Edit, so skills never reach for the CLI version. Grep across `.claude/skills/` confirms zero callers. Drop the subcommand and its tests rather than keeping a shell-only fallback nobody uses.

New forge primitive: `ether-forge preflight`. Skill worktree sessions currently hit two preventable failures — dirty `main` before `EnterWorktree` (stranded edits that break the later ff-merge) and a worktree base that's behind `main`'s HEAD (forces a rebase dance at merge time). Both checks are identical across `/dev`, `/groom`, and `/roadmap`, so they belong in forge rather than copy-pasted into each skill.

`ether-forge preflight` should:

- Refuse with a listing if `main`'s working tree is dirty.
- Refuse if the current branch's merge base with `main` is not `main`'s HEAD (worktree is behind).
- Optionally (matching `/dev` step 3): refuse if a `T*` branch already exists for a passed task ID.

Each skill's setup phase then becomes a one-line `ether-forge preflight` call before `EnterWorktree`, keeping the invariants single-sourced.

Non-goals: new `ether-forge` subcommands (the surface is already sufficient), changes to `/dev` or `/groom` (they're saturated), skill rewrites beyond wiring the missing calls.

### Ordering

1. `/close` wiring (S) — one-line edit, immediate value in every wrap-up.
2. `/roadmap` grounding (S) — `status` + `list` preamble; `groom --json` optional follow-up.
3. `/brainstorm` wiring (S) — `search`/`deps` as idea-validation primitives.
4. `/backlog` fill-in (S) — expose `get`, `search`, `deps`, `next` as first-class verbs.
5. Remove `ether-forge worktree` subcommand + tests (S, Rust).
6. `ether-forge preflight` subcommand (M, Rust) — blocks on #5 only in the sense that both touch the CLI surface; otherwise independent.

Items 1–5 are S-sized and independent. Item 6 is the only Rust work beyond a subcommand deletion.

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
