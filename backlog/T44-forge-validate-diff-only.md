---
id: T44
title: ether-forge validate --diff-only — scope checks to changed files
size: S
status: ready
priority: 3
---

Extend `ether-forge validate` with a `--diff-only [T<n>]` flag that limits checks to files touched by `git diff main` (or the task-scoped worktree diff when an id is given). Runs the reviewer-relevant subset: new unsafe blocks missing `// SAFETY:` comments, new `HashMap`/`HashSet` iteration reaching output paths, and new `TODO`/`FIXME` markers.

Merges the reviewer's manual "read rules → diff → translate to searches" loop into a single forge call.

## Sub-steps

- [x] Parse `git diff main` to extract added/modified file paths
- [x] Add `--diff-only` flag to `cmd/validate.rs`
- [x] Implement SAFETY-comment check on new unsafe blocks
- [x] Implement hashmap-iteration check on changed files
- [x] Implement TODO/FIXME scan on added lines only
- [x] Tests with fixture diffs covering each check
