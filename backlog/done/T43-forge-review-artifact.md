---
id: T43
title: ether-forge review-artifact — write reviewer JSON artifact
size: S
status: done
priority: 2
commit: 765b3af
---

Add `ether-forge review-artifact --task T<n> [--blocker file:line:msg]... [--nit file:line:msg]...` subcommand. Writes `target/.ether-forge/review-T<n>.json` with the canonical `{blockers: [...], nits: [...]}` schema, creating parent directories as needed and validating every entry has `file`/`line`/`message`.

Eliminates the reviewer subagent's hand-rolled `mkdir -p` + `Write` dance, and guarantees the downstream commit-gate contract stays stable by making the schema mechanical instead of prose-driven.
