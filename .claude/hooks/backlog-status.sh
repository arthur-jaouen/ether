#!/usr/bin/env bash
# backlog-status.sh — compact backlog summary for SessionStart context injection.
#
# Thin wrapper around `ether-forge status` (the canonical implementation).
# Falls back to an awk-based parser only if the binary is not yet built —
# this can happen on the very first session before the bootstrap hook has
# compiled the workspace. Both code paths must produce byte-identical
# output so /dev's "trust the hook over forge" rule never fires on a real
# divergence.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
backlog_dir="$repo_root/backlog"

if command -v ether-forge >/dev/null 2>&1; then
    exec ether-forge status --backlog-dir "$backlog_dir"
fi

# ---------- Fallback: cold-start path, no ether-forge binary yet ----------
# Mirrors the rendering in crates/ether-forge/src/cmd/status.rs::render so
# that swapping between the two paths is invisible to downstream consumers.

if [ ! -d "$backlog_dir" ]; then
    echo "backlog: 0 tasks — 0 ready, 0 blocked, 0 draft, 0 done"
    echo "next: (none)"
    exit 0
fi

shopt -s nullglob
files=("$backlog_dir"/T*.md)
shopt -u nullglob

ready=0
blocked=0
draft=0
done_=0

# best_ready: lowest (priority, numeric_id) where missing priority sorts last.
best_pri=""
best_id=""
best_title=""
best_tn=""

for f in "${files[@]}"; do
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
        # Match Task::pick_key: missing priority sorts as u32::MAX.
        pri="${priority:-4294967295}"
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

total=${#files[@]}
echo "backlog: $total tasks — $ready ready, $blocked blocked, $draft draft, $done_ done"
if [ -n "$best_tn" ]; then
    # Two-space separator matches `next` / `status` rendering in ether-forge.
    echo "next: $best_tn  $best_title"
else
    echo "next: (none)"
fi
