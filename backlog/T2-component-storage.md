---
id: T2
title: Component trait and sparse set storage
size: M
status: ready
---

## Sub-steps

- [ ] Define `Component` trait in `crates/ether-core/src/component.rs` — marker trait with `'static + Send + Sync` bounds
- [ ] Implement `ComponentId` — wrapper around `TypeId` with a stable numeric id via a global registry
- [ ] Implement `SparseSet<T>` in `crates/ether-core/src/storage/sparse_set.rs` — sparse array (entity index → dense index) + dense array of (Entity, T)
- [ ] SparseSet methods: `insert(entity, value)`, `remove(entity) -> Option<T>`, `get(entity) -> Option<&T>`, `get_mut(entity) -> Option<&mut T>`, `iter() -> impl Iterator`
- [ ] Wire `World::insert<T>`, `World::remove<T>`, `World::get<T>`, `World::get_mut<T>` using type-erased component storage map
- [ ] Unit tests: insert/get/remove, iteration order, swap-remove correctness, stale entity rejection
