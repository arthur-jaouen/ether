---
id: T22
title: ether-forge grep subcommand with shared recipes
size: S
status: ready
priority: 16
---

## Sub-steps

- [ ] Implement `grep <recipe>` in `crates/ether-forge/src/cmd/grep.rs` — shells out to `rg` with the recipe's pattern and optional path filter; deterministic (sorted) output
- [ ] Create `.claude/rules/grep/` with starter recipes as YAML files: `unsafe-without-safety.yml`, `hashmap-iter.yml`, `todo.yml`, `dead-code.yml` (each: name, pattern, optional path glob)
- [ ] Wire into `cmd/mod.rs` and clap; list available recipes on `grep --list`
- [ ] Integration test: invoke against a fixture tree and assert expected matches plus missing-recipe error path
