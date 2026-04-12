---
id: T18
title: ether-forge find and rewrite subcommands (ast-grep wrapper)
size: S
status: ready
priority: 12
---

## Sub-steps

- [ ] Implement `find <pattern> [--lang rust] [--rule <name>]` in `crates/ether-forge/src/cmd/find.rs` — shells out to `ast-grep run -p <pattern>`; with `--rule`, resolve from `.claude/rules/sg/<name>.yml`
- [ ] Implement `rewrite <pattern> --to <replacement>` in `crates/ether-forge/src/cmd/rewrite.rs` — shells out to `ast-grep run -p <pattern> --rewrite <replacement> -U`
- [ ] Create `.claude/rules/sg/` with a starter rule file `no-unwrap-in-core.yml` targeting `$X.unwrap()` in `crates/ether-core/`
- [ ] Document the `cargo install ast-grep` prerequisite alongside the nextest note from T17
- [ ] Integration test: invoke `find` against a fixture file and assert expected match output
