---
id: T45
title: ether-forge rules cat — concatenate CLAUDE.md and rule files
size: S
status: draft
---

Add `ether-forge rules cat` (and a sibling `ether-forge rules list`) subcommand. `cat` prints `CLAUDE.md` followed by every `.claude/rules/**/*.md` file on stdout, each with a short `# --- <path> ---` separator. `list` just prints the paths.

Replaces the reviewer subagent's step 1 — reading `CLAUDE.md` and every rule file individually — with a single forge call.

## Sub-steps

- [ ] New `cmd/rules.rs` with `cat` and `list` subcommands
- [ ] Deterministic ordering (sort rule paths lexicographically)
- [ ] Handle missing `.claude/rules/` directory gracefully
- [ ] Tests with a fixture rules tree
- [ ] Update reviewer.md step 1 to use `ether-forge rules cat`
