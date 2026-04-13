---
id: T44
title: ether-forge validate --diff-only — scope checks to changed files
size: S
status: done
priority: 3
commit: b0d536c
---

Extend `ether-forge validate` with a `--diff-only [T<n>]` flag that limits checks to files touched by `git diff main` (or the task-scoped worktree diff when an id is given). Runs the reviewer-relevant subset: new unsafe blocks missing `// SAFETY:` comments, new `HashMap`/`HashSet` iteration reaching output paths, and new `TODO`/`FIXME` markers.

Merges the reviewer's manual "read rules → diff → translate to searches" loop into a single forge call.
