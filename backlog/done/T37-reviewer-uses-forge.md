---
id: T37
title: reviewer agent uses ether-forge diff and task context
size: M
status: draft
---

The `reviewer` subagent (`.claude/agents/reviewer.md`) currently runs raw
`git diff main` and its tool allowlist only permits `Bash(git diff:*)`. Two
gaps:

- `ether-forge diff` is the review-scoped variant (strips lockfiles, caps
  size) — reviewer should use it instead of raw `git diff main`.
- The `/dev` skill prompt already instructs the reviewer to run
  `ether-forge task T<n> --context` for goal + linked ROADMAP section, but
  the agent's allowlist doesn't permit `ether-forge`, so that instruction
  cannot execute today.

## Sub-steps

- [ ] Add `Bash(ether-forge:*)` to the reviewer `tools:` frontmatter in `.claude/agents/reviewer.md`
- [ ] Replace the `git diff main` instruction with `ether-forge diff` in the "On every invocation" section
- [ ] Add an explicit step to run `ether-forge task T<n> --context` for goal context
- [ ] Drop `Bash(git diff:*)` from the allowlist if no longer needed (keep `git status` for sanity checks)
- [ ] Verify the `/dev` skill's reviewer prompt still reads consistently with the updated agent doc
