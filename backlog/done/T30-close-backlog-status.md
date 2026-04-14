---
id: T30
title: /close includes backlog status in wrap-up
size: S
status: done
priority: 2
commit: 0e7a684
---

Wire `ether-forge status` into the `/close` skill so every session wrap-up ends with a concrete backlog delta instead of prose. Part of Phase 0.5.6 — analysis skills should query `ether-forge` as a shared state layer, not re-read files.
