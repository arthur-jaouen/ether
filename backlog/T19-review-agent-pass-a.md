---
id: T19
title: Review agent Pass A — custom reviewer agent and skill quick wins
size: S
status: ready
priority: 13
---

## Sub-steps

- [ ] Create `.claude/agents/reviewer.md` — pinned `model: haiku`, minimal tool allowlist (`Read`, `Grep`, `Glob`, `Bash(git diff:*)`), terse system prompt instructing the agent to read `CLAUDE.md` + `.claude/rules/*.md` at invocation so rules stay single-sourced
- [ ] Rewrite `.claude/skills/dev/SKILL.md` self-review step — spawn `subagent_type: reviewer` with the task ID only; the subagent `cd`s into the worktree and runs `git diff main` itself instead of receiving the diff inline
- [ ] Add trivial-diff skip in `.claude/skills/dev/SKILL.md` — if `git diff main --stat` shows <30 changed lines and no `unsafe` / `HashMap` / new test files, self-review inline in the main loop instead of spawning
- [ ] Manual end-to-end test: run `/dev` against a small live task and confirm the reviewer agent fires, reads rules, and returns findings without the diff entering the parent context
