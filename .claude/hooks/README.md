# Claude Code hooks

Shell scripts invoked by the Claude Code harness per the matchers in
`.claude/settings.json`.

| Hook | Event | Purpose |
|------|-------|---------|
| `guard-bash.sh` | PreToolUse (Bash) | Block dangerous shell invocations |
| `fmt-rs.sh` | PostToolUse (Edit\|Write) | Auto-format touched `.rs` files |
| `bootstrap.sh` | SessionStart | Build+link `ether-forge`, install `cargo-nextest` prebuilt binary if absent — idempotent, quiet on the happy path |
| `backlog-status.sh` | SessionStart | Inject compact backlog summary into context |
| `validate.sh` | SessionEnd | Backlog integrity check (unique IDs, depends_on refs) |

## Implementation notes

`backlog-status.sh` is a thin wrapper that `exec`s `ether-forge status`.
The SessionStart pipeline runs `bootstrap.sh` first (see
`.claude/settings.json`), which builds and symlinks the binary, so the
happy path is always the `exec` branch. If bootstrap's build failed, the
wrapper emits a stub summary rather than hard-failing the SessionStart
pipeline.

`validate.sh` is still a bash fallback; it will be replaced by
`ether-forge validate` (after T10).
