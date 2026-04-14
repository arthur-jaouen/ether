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

`backlog-status.sh` is a thin wrapper that `exec`s `ether-forge status`
when the binary is on `$PATH`. It keeps an awk-based fallback for cold
sessions (before the bootstrap hook compiles the workspace) — the fallback
mirrors `crates/ether-forge/src/cmd/status.rs::render` byte-for-byte so
swapping between the two paths is invisible. Once the web bootstrap is
guaranteed to run before SessionStart consumers, the fallback can be
deleted and the hook collapsed into a direct `ether-forge status`
invocation in `.claude/settings.json`.

`validate.sh` is still a bash fallback; it will be replaced by
`ether-forge validate` (after T10).
