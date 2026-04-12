---
id: T11
title: install-hooks subcommand and GitHub Actions CI
size: S
status: blocked
depends_on:
  - T7
priority: 8
---

## Sub-steps

- [ ] `install-hooks` in `crates/ether-forge/src/cmd/install_hooks.rs` — write `.git/hooks/pre-commit` that invokes `ether-forge check`; idempotent (detects and replaces its own marker)
- [ ] Create `.github/workflows/ci.yml` — Ubuntu runner, cache cargo registry + target, run `cargo run -p ether-forge -- check`
- [ ] CI triggers on `push` to main and `pull_request`
- [ ] Smoke-test the hook installer in a fresh temp repo inside an integration test
