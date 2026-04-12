---
id: T11
title: install-hooks subcommand
size: S
status: ready
priority: 8
---

## Sub-steps

- [x] `install-hooks` in `crates/ether-forge/src/cmd/install_hooks.rs` — write `.git/hooks/pre-commit` that invokes `ether-forge check`; idempotent (detects and replaces its own marker)
- [x] Smoke-test the hook installer in a fresh temp repo inside an integration test
