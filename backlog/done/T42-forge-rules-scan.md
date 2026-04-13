---
id: T42
title: ether-forge rules-scan — run all grep recipes against worktree diff
size: M
status: done
priority: 4
commit: c376665
---

Add `ether-forge rules-scan [T<n>]` subcommand. Auto-discovers every recipe under `.claude/rules/grep/*.yml`, runs each against the current worktree's `git diff main` (or a task-scoped diff when an id is supplied), and emits a `{recipe_name: [matches]}` JSON map on stdout.

Eliminates the reviewer subagent's repetitive pattern of reading each rule file then issuing separate `Grep` calls — one forge call replaces N.
