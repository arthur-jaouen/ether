---
id: T38
title: ether-forge merge subcommand for skill wrap-up
size: L
status: done
priority: 3
commit: 59697d1
---

Skill wrap-up (step 23 of `/dev` and equivalents in `/groom`, `/roadmap`) strings together four raw git calls that each have a known failure mode: ff-merge refuses when `main` advanced during the session, `git worktree remove` errors when the directory is already gone, and `git branch -d` refuses on "not fully merged" branches with identical content. Collapse the dance into one forge primitive — the exit-side mirror of `preflight`.

See ROADMAP Phase 0.5.7 for the full spec.
