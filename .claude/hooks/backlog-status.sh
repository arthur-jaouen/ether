#!/usr/bin/env bash
# backlog-status.sh — compact backlog summary for SessionStart context injection.
# Temporary — swap to `ether-forge status` once T6 lands.
#
# Parses YAML frontmatter in backlog/*.md to count tasks by status and
# identify the next ready task (lowest priority, then lowest T<n> ID).
# Output is ≤10 lines, plain text, suitable for injection into a prompt.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
backlog_dir="$repo_root/backlog"

if [ ! -d "$backlog_dir" ]; then
    echo "no tasks"
    exit 0
fi

shopt -s nullglob
files=("$backlog_dir"/T*.md)
shopt -u nullglob

if [ ${#files[@]} -eq 0 ]; then
    echo "no tasks"
    exit 0
fi

ready=0
blocked=0
draft=0
done_=0

# best_ready: "<priority> <id_num> <title>" for lowest (priority, id)
best_pri=""
best_id=""
best_title=""
best_tn=""

for f in "${files[@]}"; do
    # Extract frontmatter (between first two --- lines)
    fm=$(awk '/^---$/{n++; next} n==1{print} n>=2{exit}' "$f")
    status=$(printf '%s\n' "$fm" | awk -F': *' '/^status:/ {print $2; exit}')
    id=$(printf '%s\n' "$fm" | awk -F': *' '/^id:/ {print $2; exit}')
    title=$(printf '%s\n' "$fm" | awk '/^title:/ {sub(/^title: */, ""); print; exit}')
    priority=$(printf '%s\n' "$fm" | awk -F': *' '/^priority:/ {print $2; exit}')

    case "$status" in
        ready)   ready=$((ready + 1)) ;;
        blocked) blocked=$((blocked + 1)) ;;
        draft)   draft=$((draft + 1)) ;;
        done)    done_=$((done_ + 1)) ;;
    esac

    if [ "$status" = "ready" ]; then
        # Normalize missing priority to a large number so explicit priorities win
        pri="${priority:-9999}"
        id_num="${id#T}"
        if [ -z "$best_pri" ] \
            || [ "$pri" -lt "$best_pri" ] \
            || { [ "$pri" -eq "$best_pri" ] && [ "$id_num" -lt "$best_id" ]; }; then
            best_pri="$pri"
            best_id="$id_num"
            best_title="$title"
            best_tn="$id"
        fi
    fi
done

total=$((ready + blocked + draft))
if [ "$total" -eq 0 ]; then
    echo "no tasks"
    exit 0
fi

echo "backlog: $ready ready, $blocked blocked, $draft draft"
if [ -n "$best_tn" ]; then
    echo "next: $best_tn — $best_title"
else
    echo "next: none ready (run /groom)"
fi
