---
id: T17
title: Rewrite ether-forge check for lean output and nextest
size: S
status: done
priority: 4
commit: e69886b
---

Lean `ether-forge check`: clippy (short, all-targets) → nextest → doctests,
with `CARGO_TERM_COLOR=never`. Adds a README documenting the `cargo-nextest`
prerequisite.
