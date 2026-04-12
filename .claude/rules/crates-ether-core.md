---
globs: crates/ether-core/**
---

# ether-core rules

## Safety

- Every `unsafe` block must have a `// SAFETY:` comment explaining the invariant
- Prefer safe abstractions. Only use unsafe when benchmarks prove necessity.
- All unsafe code must have tests exercising boundary conditions (empty, max capacity, after remove)

## Performance

- Component storage must be cache-friendly — prefer contiguous arrays over pointer-chasing
- Hot paths (query iteration, component access) must avoid allocations
- Use `#[inline]` on small accessor methods in storage traits

## Invariants

- Entity generations must be monotonically increasing per index slot
- Sparse set: dense array indices must always be valid — maintain invariant on remove (swap-remove)
- ComponentId assignment must be deterministic (use TypeId-based registration)

## Testing

- Shared test helpers go in `tests/common/mod.rs`
- Benchmark fixtures go in `benches/`
