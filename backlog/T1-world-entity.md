---
id: T1
title: World and Entity types
size: M
status: ready
---

## Sub-steps

- [ ] Define `Entity` struct in `crates/ether-core/src/entity.rs` — u32 index + u32 generation, implement `Copy`, `Clone`, `Eq`, `Hash`, `Debug`
- [ ] Implement `EntityAllocator` — free list for recycled indices, generation tracking per slot
- [ ] Define `World` struct in `crates/ether-core/src/world.rs` — owns the allocator
- [ ] Implement `World::spawn() -> Entity`, `World::despawn(entity)`, `World::is_alive(entity) -> bool`
- [ ] Wire up `mod entity; mod world;` in `crates/ether-core/src/lib.rs` with public re-exports
- [ ] Unit tests: spawn, despawn, respawn reuses index with bumped generation, is_alive returns false for despawned
