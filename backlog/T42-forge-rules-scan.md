---
id: T42
title: ether-forge rules-scan — run all grep recipes against worktree diff
size: M
status: ready
priority: 4
---

Add `ether-forge rules-scan [T<n>]` subcommand. Auto-discovers every recipe under `.claude/rules/grep/*.yml`, runs each against the current worktree's `git diff main` (or a task-scoped diff when an id is supplied), and emits a `{recipe_name: [matches]}` JSON map on stdout.

Eliminates the reviewer subagent's repetitive pattern of reading each rule file then issuing separate `Grep` calls — one forge call replaces N.

## Sub-steps

- [ ] New `cmd/rules_scan.rs` that enumerates `.claude/rules/grep/*.yml`
- [ ] Parse each recipe (reuse existing grep recipe loader if present)
- [ ] Run every recipe against `git diff main` output and collect matches
- [ ] Serialize results as `{recipe: [{file, line, text}]}` JSON to stdout
- [ ] Unit tests with a fixture recipe set and synthetic diff
- [ ] Wire into `cmd/mod.rs` and update help output
