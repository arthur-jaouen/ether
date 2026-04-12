---
name: roadmap
description: Plan what's next for Ether ECS. Audits current state, discusses direction with the user, and writes ROADMAP.md updates.
argument-hint: [area or topic, e.g. "storage", "query system", "scheduling"]
---

# Roadmap

Plan what's next for an Ether area. **Conversation with structured input** — audit, discuss, write.

## Phase 1 — Audit (silent, before first response)

1. `cd /home/arthur/ether`
2. **Ground in backlog state first** — before reading anything else, run `ether-forge status` and `ether-forge list` so every subsequent claim is anchored to what's actually ready, blocked, or in flight. This is read-only and runs outside any worktree.
3. **Optional drift check:** `ether-forge groom --json` (dry-run by default) surfaces coverage gaps between `ROADMAP.md` and the backlog. Use it when the topic might already be partially covered.
4. **Parallel gather** (after grounding):
   - Read `ROADMAP.md`
   - Scan `backlog/*.md` and `backlog/done/*.md` for topic-relevant tasks
   - `git log --oneline -15`
   - Topic-relevant: run benchmarks, check test coverage, count implementations
5. Read only files directly relevant to the topic. Don't read every file upfront.

## Phase 2 — Present findings + discuss

6. Open with the most interesting finding (1-2 sentences) and one question.
7. Keep responses concise (3-8 lines). Be opinionated. One thread at a time.
8. When external knowledge is needed (algorithms, crate options):
   - Ask the user first: "I think we need to look into X — want me to research it?"
   - If yes, launch 2-3 focused parallel agents with specific questions
9. Explore code deeper only when conversation demands it.

## Phase 3 — Write

When conversation converges:

10. `ether-forge preflight` — refuses if `main` is dirty or the current branch is behind `main`'s HEAD. Fix whatever it reports before entering the worktree. Skip if already inside a worktree.
11. Call `EnterWorktree` with `name: "roadmap-<topic>"` so every tool (including `Edit` on `ROADMAP.md`) resolves against the isolated worktree. Skip this step if the session is already inside a worktree — `EnterWorktree` refuses to nest, so edit in place.
12. Ask: "Want me to write this into ROADMAP.md?"
13. Update `ROADMAP.md` — replace/update the relevant section with concrete numbers.
14. Tell user: "Run `/groom <section>` to generate tasks from this."
15. Commit in worktree.
16. Ask if the user wants to merge into `main`. On confirmation:
    - `ExitWorktree` with `action: "keep"` to return the session to the main checkout.
    - `ether-forge merge roadmap-<topic>` — rebases, ff-merges, removes the worktree, and deletes the branch. If the user declines, leave the worktree intact.

## Conversation style

- Be concise (3-5 sentences per turn during discussion)
- Be opinionated — take a position, don't hedge
- One thread at a time
- Build on what the user says
- If an idea has a flaw, say so directly

## Asking questions

- **Default to `AskUserQuestion`** whenever there are identifiable options (2-3 options, never 4 — that feels like a quiz). The user can always pick "Other" for freeform input.
- **Use `preview`** on options when comparing approaches — show a code snippet or ASCII diagram so the user can visually compare before picking.
- **Plain text only** for truly open-ended prompts where options would box the user in.

## What NOT to do

- Don't read the entire crate upfront
- Don't launch broad research agents without asking
- Don't write ROADMAP without asking
- Don't produce structured dumps in the first response
- Don't start implementing — this is planning, not building
