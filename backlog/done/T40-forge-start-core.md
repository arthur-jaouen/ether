---
id: T40
title: ether-forge start subcommand — core
size: M
status: done
priority: 1
commit: 8738f06
---

Entry-side mirror of T38 (`merge`). Ship a working `ether-forge start T<n>` that collapses the `/dev` kickoff dance (`get`, `check`, `preflight`, worktree add, fetch+rebase) into one primitive. This task covers the happy path only; edge-case flags and skill wiring land in the follow-up.
