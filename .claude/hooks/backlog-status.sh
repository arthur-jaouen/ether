#!/usr/bin/env bash
# backlog-status.sh — compact backlog summary for SessionStart context injection.
#
# Thin wrapper around `ether-forge status` (the canonical implementation).
# The bootstrap SessionStart hook builds and symlinks the binary before this
# one runs, so the happy path always takes the `exec` branch. If the build
# failed, emit a clear stub instead of hard-failing the SessionStart pipeline.

set -euo pipefail

if ! command -v ether-forge >/dev/null 2>&1; then
    echo "backlog: ether-forge not built — bootstrap.sh likely failed"
    echo "next: (unavailable)"
    exit 0
fi

exec ether-forge status
