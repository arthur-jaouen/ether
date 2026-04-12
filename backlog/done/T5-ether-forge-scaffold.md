---
id: T5
title: Scaffold ether-forge crate and frontmatter parser
size: M
status: done
priority: 1
---

Scaffolded the `ether-forge` binary crate with `clap` CLI stub and a
`Task` frontmatter parser supporting `Task::load` and `Task::load_all`
(deterministic sort by numeric ID). Unit tests cover valid parsing,
missing/malformed frontmatter, dependencies, and round-trip.
