---
id: T47
title: rules-scan supports ignore markers and exclude_paths
size: S
status: ready
priority: 8
---

Add a mechanism for `ether-forge rules-scan` to suppress false positives on known meta-mentions, so it can run in CI without flagging the checker's own documentation and test fixtures.

**Problem:** `rules-scan` grep-matches every added line against each recipe, which means a file that *describes* what the checker looks for (e.g. `crates/ether-forge/src/cmd/validate.rs` or the future `cmd/rules_scan.rs` itself) surfaces as dozens of findings. Observed in T44's smoke test: 23 findings, all self-references.

## Design

Two suppression knobs, both opt-in per recipe:

1. **Line marker:** any added line containing the literal `rules-scan: ignore` (in any comment shape) is skipped by every recipe. Cheap escape hatch for one-off doc comments.
2. **Recipe-level `exclude_paths:` list** in the recipe YAML — e.g.
   ```yaml
   name: todo
   pattern: "\\b(TODO|FIXME|XXX|HACK)\\b"
   path: crates
   exclude_paths:
     - crates/ether-forge/src/cmd/rules_scan.rs
     - crates/ether-forge/src/cmd/validate.rs
   ```
   Each entry is a path prefix matched against `DiffFile.path`.

## Sub-steps

- [ ] Extend `Recipe` struct in `cmd/grep.rs` with `#[serde(default)] exclude_paths: Vec<String>`
- [ ] `cmd/rules_scan.rs::match_recipe` — skip files whose path starts with any `exclude_paths` prefix
- [ ] `cmd/rules_scan.rs::match_recipe` — skip individual added lines containing `rules-scan: ignore`
- [ ] Tests: one recipe with `exclude_paths` excluding a hit file, one diff with an `ignore` marker
- [ ] Update the 4 shipped recipes under `.claude/rules/grep/` with `exclude_paths` for `validate.rs` and `rules_scan.rs`
