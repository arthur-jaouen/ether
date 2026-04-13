---
id: T46
title: ether-forge review — umbrella command chaining scan + validate + artifact
size: M
status: blocked
depends_on:
  - T42
  - T44
---

Add `ether-forge review [T<n>]` umbrella subcommand that chains `rules-scan` + `validate --diff-only` + `review-artifact` into one call, producing the canonical `target/.ether-forge/review-T<n>.json` from mechanical checks alone.

Lets `/dev` skip spawning the reviewer subagent for diffs where only mechanical rule enforcement is needed, reserving the LLM reviewer for semantic judgment.

## Sub-steps

- [ ] New `cmd/review.rs` that invokes the three underlying commands in-process
- [ ] Merge findings into the canonical blockers/nits shape
- [ ] Classify mechanical findings (SAFETY missing = blocker, TODO = nit, etc.)
- [ ] Write the artifact via the `review-artifact` helper
- [ ] Tests asserting the merged shape for a fixture diff
- [ ] Document when to prefer the umbrella vs the subagent in reviewer.md
