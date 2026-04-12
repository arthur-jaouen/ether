---
id: T3
title: Derive Component macro
size: S
status: blocked
depends_on:
  - T2
---

## Sub-steps

- [ ] Implement `#[derive(Component)]` in `crates/ether-macros/src/lib.rs` — generates `impl Component for T {}` for structs
- [ ] Emit `compile_error!` for enums and unions
- [ ] Add trybuild pass test: struct with named fields derives Component
- [ ] Add trybuild fail test: enum and union rejected with clear error message
