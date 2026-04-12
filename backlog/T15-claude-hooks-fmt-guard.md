---
id: T15
title: Claude hooks — PostToolUse fmt and PreToolUse destructive guard
size: S
status: ready
priority: 2
---

## Sub-steps

- [ ] Add `hooks.PostToolUse` entry in `.claude/settings.json` matching `Edit` and `Write` on `*.rs`, running `cargo fmt -- $CLAUDE_TOOL_FILE_PATH`
- [ ] Add `hooks.PreToolUse` entry matching `Bash`, running `.claude/hooks/guard-bash.sh` which greps the command for destructive patterns and exits non-zero to block
- [ ] Destructive patterns: `git push --force`, `git push -f`, `git reset --hard`, `git branch -D`, `rm -rf`
- [ ] Print a clear block message naming the matched pattern so the model understands why it was denied
- [ ] Manual test: trigger an edit, verify fmt runs; attempt a blocked bash command, verify it is denied
