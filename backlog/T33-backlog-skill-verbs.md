---
id: T33
title: /backlog exposes get, search, deps, next as first-class verbs
size: S
status: ready
---

`/backlog` currently wraps `list`, `status`, `validate`, and `done` but re-implements task inspection, keyword search, dependency inspection, and next-pick in prose. Expose the missing `ether-forge` subcommands as first-class verbs so day-to-day CRUD converges on the CLI.

## Sub-steps

- [ ] Edit `.claude/skills/backlog/SKILL.md`: add `get`, `search`, `deps`, `next` sections mirroring the existing verb style.
- [ ] For each verb, document arg shape and one example invocation.
- [ ] Remove any inline prose that duplicated what the verbs now cover.
