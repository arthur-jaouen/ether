---
id: T41
title: ether-forge start — polish and skill wiring
size: M
status: blocked
depends_on:
  - T40
---

Follow-up to T40. Harden the `start` subcommand's edge cases and swap the skills over to use it so `start` / `merge` become the bookends of every `/dev`, `/groom`, `/roadmap` session.

## Sub-steps

- [ ] `--keep-existing` flag: if the worktree dir already exists, reuse it instead of erroring (rerun-after-interruption case).
- [ ] Regression test: `main` advanced after the branch was first created — `start` must rebase cleanly.
- [ ] Regression test: pre-existing worktree dir with `--keep-existing` — must reuse, not error.
- [ ] Update `.claude/skills/dev/SKILL.md` startup section: replace the get/check/preflight/worktree/rebase sequence with a single `ether-forge start T<n>` call.
- [ ] Update `.claude/skills/groom/SKILL.md` and `.claude/skills/roadmap/SKILL.md` kickoff sections to call `ether-forge start` where they create worktrees.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` stay green.
