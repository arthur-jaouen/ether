#!/usr/bin/env bash
# guard-bash.sh — PreToolUse hook on Bash.
# Reads hook JSON from stdin, extracts tool_input.command, and blocks
# destructive patterns by exiting 2 (stderr is shown to the model).

set -euo pipefail

command=$(jq -r '.tool_input.command // ""')

if [ -z "$command" ]; then
    exit 0
fi

patterns=(
    'git push --force'
    'git push -f'
    'git reset --hard'
    'git branch -D'
    'rm -rf'
)

for p in "${patterns[@]}"; do
    if printf '%s' "$command" | grep -qF -- "$p"; then
        printf 'guard-bash: blocked destructive pattern: %s\n' "$p" >&2
        printf 'command was: %s\n' "$command" >&2
        exit 2
    fi
done

exit 0
