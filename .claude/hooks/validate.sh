#!/usr/bin/env bash
# validate.sh — backlog integrity check for SessionEnd.
# Temporary — swap to `ether-forge validate` once T10 lands.
#
# Checks:
#   1. Every backlog/*.md has a unique `id:` in its YAML frontmatter.
#   2. Every `depends_on:` entry references an existing task (in backlog/ or backlog/done/).
#
# Exits 0 on success, 1 on any violation. Prints one issue per line to stderr.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
backlog_dir="$repo_root/backlog"
done_dir="$backlog_dir/done"

if [ ! -d "$backlog_dir" ]; then
    exit 0
fi

shopt -s nullglob
active_files=("$backlog_dir"/T*.md)
done_files=("$done_dir"/T*.md)
shopt -u nullglob

all_files=("${active_files[@]}" "${done_files[@]}")

if [ ${#all_files[@]} -eq 0 ]; then
    exit 0
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

# Pass 1: collect all known IDs (active + done).
for f in "${all_files[@]}"; do
    id=$(awk '/^---$/{n++; next} n==1 && /^id:/ {sub(/^id: */, ""); print; exit} n>=2{exit}' "$f")
    [ -n "$id" ] && printf '%s\n' "$id" >> "$tmp"
done

status=0

# Check ID uniqueness across active + done.
dupes=$(sort "$tmp" | uniq -d)
if [ -n "$dupes" ]; then
    while IFS= read -r dup; do
        echo "validate: duplicate id: $dup" >&2
        status=1
    done <<< "$dupes"
fi

known_ids=$(sort -u "$tmp")

# Pass 2: check depends_on references exist.
for f in "${active_files[@]}"; do
    fm=$(awk '/^---$/{n++; next} n==1{print} n>=2{exit}' "$f")
    # Extract items under depends_on: up to the next top-level key or end of frontmatter.
    deps=$(printf '%s\n' "$fm" | awk '
        /^depends_on:/ {in_deps=1; next}
        in_deps && /^[[:space:]]*-[[:space:]]*/ {sub(/^[[:space:]]*-[[:space:]]*/, ""); print; next}
        in_deps && /^[^[:space:]]/ {in_deps=0}
    ')
    while IFS= read -r dep; do
        [ -z "$dep" ] && continue
        if ! printf '%s\n' "$known_ids" | grep -qx "$dep"; then
            echo "validate: $(basename "$f"): depends_on references unknown task: $dep" >&2
            status=1
        fi
    done <<< "$deps"
done

if [ "$status" -eq 0 ]; then
    echo "validate: ok (${#active_files[@]} active, ${#done_files[@]} done)"
fi

exit "$status"
