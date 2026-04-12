---
id: T33
title: /backlog exposes get, search, deps, next as first-class verbs
size: S
status: done
priority: 4
commit: b198774
---

`/backlog` currently wraps `list`, `status`, `validate`, and `done` but re-implements task inspection, keyword search, dependency inspection, and next-pick in prose. Expose the missing `ether-forge` subcommands as first-class verbs so day-to-day CRUD converges on the CLI.
