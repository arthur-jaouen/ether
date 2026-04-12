---
id: T4
title: Basic query iteration
size: M
status: blocked
depends_on:
  - T2
---

## Sub-steps

- [ ] Define `Query` type in `crates/ether-core/src/query.rs` — parameterized by a tuple of component references
- [ ] Implement `WorldQuery` trait for `&T` (immutable access) and `&mut T` (mutable access)
- [ ] Implement tuple query: `Query<(&A, &B)>` iterates entities that have both A and B
- [ ] Wire `World::query<Q: WorldQuery>()` method returning an iterator
- [ ] Unit tests: single-component query, multi-component query, query skips entities missing components
- [ ] Unit test: mutable query can modify component values
