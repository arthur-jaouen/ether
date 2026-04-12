---
id: T28
title: Reviewer JSON output contract
size: S
status: ready
---

Make the reviewer agent emit a machine-readable artifact alongside its prose summary so downstream tooling (commit gate) can enforce blocker severity mechanically. Covers ROADMAP 0.5.5.3a.

## Sub-steps

- [ ] Update `.claude/agents/reviewer.md` system prompt: after the existing terse summary, write `target/.ether-forge/review-T<id>.json` with shape `{"blockers": [...], "nits": [...]}` where each entry has `{file, line, message}`.
- [ ] Ensure the reviewer's tool allowlist includes `Write` (or `Bash` mkdir + redirect) for the artifact path.
- [ ] Document the artifact path and schema in a short note at the bottom of `.claude/agents/reviewer.md` so the contract is discoverable.
- [ ] Manual verification: run `/dev` against a trivial task and confirm the JSON file is produced with the expected shape.
