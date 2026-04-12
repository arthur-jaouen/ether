---
id: T32
title: /brainstorm wires search and deps for idea validation
size: S
status: ready
priority: 6
---

`/brainstorm` never checks whether a proposed idea is already captured or blocked by existing work. Wire `ether-forge search` and `ether-forge deps` as first-class validation primitives so brainstorming surfaces adjacency instead of producing duplicates.

## Sub-steps

- [ ] Edit `.claude/skills/brainstorm/SKILL.md`: add a "before proposing" checklist that runs `ether-forge search <keywords>` for each candidate idea.
- [ ] When the search matches, run `ether-forge deps T<n>` on the hit to surface blockers and dependents before mentioning it to the user.
- [ ] Document the fallback: if no matches, the idea is genuinely new — proceed to discussion.
