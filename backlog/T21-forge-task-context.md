---
id: T21
title: ether-forge task --context subcommand (task + linked ROADMAP blob)
size: S
status: ready
priority: 15
---

## Sub-steps

- [ ] Add `task T<n> --context` in `crates/ether-forge/src/cmd/task.rs` (or extend `get.rs`) — emits frontmatter + body + the linked ROADMAP.md section as one blob
- [ ] Match the ROADMAP section by keyword fallback plus explicit section tags (shared helper with T12's section-mapping logic)
- [ ] Wire into `cmd/mod.rs` and clap
- [ ] Unit test output shape against a known task + roadmap section fixture
