---
id: T34
title: Document ether-forge worktree vs EnterWorktree preference
size: S
status: ready
---

`ether-forge worktree T<n>` is dead code inside agent-driven skills: only `EnterWorktree` re-roots Glob/Grep/Read/Edit, so the CLI subcommand is never the right pick from inside a Claude session. It remains useful for humans invoking the CLI from a shell. Document the distinction so future skill edits don't reintroduce it.

## Sub-steps

- [ ] Add a short "agent vs human" note to `BACKLOG.md` or `CLAUDE.md` (whichever is closer to the skill-authoring audience) contrasting `EnterWorktree` (session-scoped, re-roots tools) and `ether-forge worktree` (shell-scoped).
- [ ] Cross-reference from the Phase 0.5.6 section in `ROADMAP.md`.
- [ ] Grep existing skills for `ether-forge worktree` and confirm no skill still reaches for it.
