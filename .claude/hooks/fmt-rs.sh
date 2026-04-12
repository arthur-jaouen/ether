#!/usr/bin/env bash
# fmt-rs.sh — PostToolUse hook on Edit|Write.
# Reads hook JSON from stdin, extracts tool_input.file_path, and runs
# rustfmt on it when the file has a .rs extension. Non-Rust files are
# ignored. Never blocks: rustfmt failure is surfaced but exits 0.

set -uo pipefail

file_path=$(jq -r '.tool_input.file_path // ""')

case "$file_path" in
    *.rs) ;;
    *) exit 0 ;;
esac

if [ ! -f "$file_path" ]; then
    exit 0
fi

rustfmt --edition 2021 "$file_path" 2>&1 || true
exit 0
