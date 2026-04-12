# Claude Code hooks

Shell scripts invoked by the Claude Code harness per the matchers in
`.claude/settings.json`.

| Hook | Event | Purpose |
|------|-------|---------|
| `guard-bash.sh` | PreToolUse (Bash) | Block dangerous shell invocations |
| `fmt-rs.sh` | PostToolUse (Edit\|Write) | Auto-format touched `.rs` files |
| `backlog-status.sh` | SessionStart | Inject compact backlog summary into context |
| `validate.sh` | SessionEnd | Backlog integrity check (unique IDs, depends_on refs) |

## Temporary implementations

`backlog-status.sh` and `validate.sh` are bash fallbacks. They will be
replaced by `ether-forge status` (after T6) and `ether-forge validate`
(after T10) respectively — at that point the `command` fields in
`.claude/settings.json` should be updated to call the binary directly
and these scripts can be deleted.
