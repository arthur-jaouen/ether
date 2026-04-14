---
id: T48
title: validate --diff-only honors rules-scan ignore markers
size: S
status: blocked
depends_on:
  - T47
priority: 9
---

Make `ether-forge validate --diff-only` respect the same suppression mechanism as `rules-scan` (T47), so the two mechanical checkers behave consistently. Without this, a self-referential line flagged by one tool but not the other just shifts the noise around.

**Problem:** T44's smoke test against its own diff produced 23 `validate --diff-only` findings, every one a self-reference in `validate.rs` (doc comments, test fixtures). The fix has to match whatever T47 ships so reviewers only learn one escape hatch.

## Sub-steps

- [ ] `cmd/validate.rs::diff_checks` — skip lines containing `rules-scan: ignore`
- [ ] Load any `exclude_paths:` from a shared config surface (reuse whatever T47 exposes — likely a small helper in `cmd/grep.rs` that returns a `BTreeMap<recipe_name, exclude_paths>`)
- [ ] Have `diff_checks` consult a hardcoded exclude list for `validate.rs` itself, OR — if T47's config surface is generic enough — reuse it by keying on synthetic recipe names like `unsafe-safety`, `hashmap-iter`, `todo-fixme`
- [ ] Tests: fixture diff with an ignore marker and an excluded path, assert no findings
- [ ] Smoke test: run `ether-forge validate --diff-only` against the current T44/T48 diff and confirm the meta-noise is gone
