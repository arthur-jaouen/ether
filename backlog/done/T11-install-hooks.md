---
id: T11
title: install-hooks subcommand
size: S
status: done
priority: 8
commit: c886fa2
---

`ether-forge install-hooks` writes `.git/hooks/pre-commit` invoking
`ether-forge check`. Idempotent via a marker line; refuses to clobber
foreign hooks and rejects linked worktrees.
