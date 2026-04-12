---
id: T35
title: ether-forge preflight subcommand for worktree sessions
size: L
status: done
priority: 2
commit: 1501258
---

Skill worktree sessions currently hit two preventable failures: (1) dirty `main` before `EnterWorktree` strands edits that break the later ff-merge, (2) a worktree base behind `main`'s HEAD forces a rebase dance at merge time. Both checks are identical across `/dev`, `/groom`, and `/roadmap`, so they belong in one forge primitive rather than copy-pasted into each skill.
