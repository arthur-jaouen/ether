#!/usr/bin/env bash
# bootstrap.sh — make the Ether workspace immediately usable in a fresh
# Claude Code on the web session. Runs on SessionStart:startup.
#
# Ensures three things are ready before the /dev workflow starts:
#
#   1. `ether-forge` is on PATH. The binary is built from this workspace's
#      own crate, so a release build + symlink into ~/.cargo/bin is enough.
#   2. `cargo-nextest` exists. `ether-forge check` invokes it directly; a
#      missing binary fails the whole verification suite.
#   3. The baseline passes. We don't run `ether-forge check` here (too slow
#      for a SessionStart hook) — just surface the readiness summary.
#
# All steps are idempotent. The hook is intentionally quiet on the happy
# path: one line per step so the resume log stays compact.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

bin_dir="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$bin_dir"

log() { printf 'bootstrap: %s\n' "$*"; }

# ---- 1. ether-forge on PATH ---------------------------------------------

ensure_ether_forge() {
    local target="$repo_root/target/release/ether-forge"
    local link="$bin_dir/ether-forge"

    if [ ! -x "$target" ]; then
        log "building ether-forge (release) — one-time per fresh target/"
        if ! cargo build -p ether-forge --release --quiet 2>&1 | sed 's/^/bootstrap:   /'; then
            log "cargo build failed — /dev will not work until fixed"
            return 1
        fi
    fi

    if [ -L "$link" ] && [ "$(readlink "$link")" = "$target" ]; then
        return 0
    fi
    ln -sf "$target" "$link"
    log "linked $link -> target/release/ether-forge"
}

# ---- 2. cargo-nextest ---------------------------------------------------

ensure_nextest() {
    if command -v cargo-nextest >/dev/null 2>&1; then
        return 0
    fi
    log "installing cargo-nextest (prebuilt binary)"

    local tmp
    tmp=$(mktemp -d)
    # shellcheck disable=SC2064
    trap "rm -rf '$tmp'" EXIT

    local arch
    arch=$(uname -m)
    local triple
    case "$arch" in
        x86_64)  triple="x86_64-unknown-linux-gnu" ;;
        aarch64) triple="aarch64-unknown-linux-gnu" ;;
        *)       log "unsupported arch $arch — install cargo-nextest manually"; return 1 ;;
    esac

    local version
    version=$(
        curl -sSL https://api.github.com/repos/nextest-rs/nextest/releases/latest \
            | awk -F'"' '/"tag_name":/ {print $4; exit}' \
            | sed 's/^cargo-nextest-//'
    )
    if [ -z "$version" ]; then
        log "could not resolve latest nextest version — install manually"
        return 1
    fi

    local url="https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${version}/cargo-nextest-${version}-${triple}.tar.gz"
    if ! curl -fsSL -o "$tmp/nextest.tar.gz" "$url"; then
        log "download failed ($url) — install manually"
        return 1
    fi
    tar -xzf "$tmp/nextest.tar.gz" -C "$bin_dir"
    log "installed cargo-nextest $version"
}

# ---- 3. report ----------------------------------------------------------

ensure_ether_forge || true
ensure_nextest || true

if command -v ether-forge >/dev/null 2>&1 && command -v cargo-nextest >/dev/null 2>&1; then
    log "ready: ether-forge $(ether-forge --version 2>/dev/null | awk '{print $2}'), $(cargo nextest --version 2>/dev/null | head -1 | awk '{print $2}')"
fi
