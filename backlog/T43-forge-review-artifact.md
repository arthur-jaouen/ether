---
id: T43
title: ether-forge review-artifact — write reviewer JSON artifact
size: S
status: ready
priority: 2
---

Add `ether-forge review-artifact --task T<n> [--blocker file:line:msg]... [--nit file:line:msg]...` subcommand. Writes `target/.ether-forge/review-T<n>.json` with the canonical `{blockers: [...], nits: [...]}` schema, creating parent directories as needed and validating every entry has `file`/`line`/`message`.

Eliminates the reviewer subagent's hand-rolled `mkdir -p` + `Write` dance, and guarantees the downstream commit-gate contract stays stable by making the schema mechanical instead of prose-driven.

## Sub-steps

- [x] New `cmd/review_artifact.rs` with clap args for task id + repeated `--blocker`/`--nit`
- [x] Parse `file:line:msg` entries, reject malformed ones with clear errors
- [x] `mkdir -p` `target/.ether-forge/` and write the JSON file
- [x] Also accept `--from-stdin` to read a pre-built JSON payload and validate+normalize it
- [x] Tests covering empty arrays, malformed entries, and stdin mode
- [x] Update reviewer.md to call `ether-forge review-artifact` instead of raw Write
