---
id: T27
title: "ether-forge: resolve repo root via git rev-parse --show-toplevel"
size: S
status: done
priority: 2
commit: 4ed01b4
---

# ether-forge: resolve repo root via git rev-parse --show-toplevel

Make `ether-forge` robust to being invoked from any subdirectory of the repo (including nested worktree paths). Today every subcommand defaults `--backlog = "backlog"` relative to cwd, and `cmd/worktree.rs:32` uses `current_dir()` as the repo root. A helper that shells to `git rev-parse --show-toplevel` once and threads the result through the CLI removes the cwd dependency.

Independent of T24/T25/T26 — those fix the worktree drift at the skill layer; this one keeps forge correct on its own.
