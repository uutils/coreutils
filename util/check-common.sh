#!/bin/bash
#
# Shared setup for the util/check-*.sh syscall verification scripts
# (check-safe-traversal.sh, check-toctou.sh, check-libc-interposition.sh).
#
# Source it after setting CHECK_UTILS to the utilities the caller exercises:
#
#     CHECK_UTILS="mkfifo touch head"
#     . "$(dirname "${BASH_SOURCE[0]}")/check-common.sh"
#
# It provides $PROJECT_ROOT, a $TEMP_DIR cleaned up on exit, fail_immediately(),
# require_command(), and util_cmd()/have_util() to resolve a utility to a
# runnable command whether the tree was built as individual binaries or as the
# multicall binary.

: "${PROFILE:=release-small}"
export PROFILE

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="$PROJECT_ROOT/target/$PROFILE"
TEMP_DIR=$(mktemp -d)

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

fail_immediately() {
    echo "❌ FAILED: $1"
    # Only the strace-based checks leave logs behind; mention them if they exist.
    if compgen -G "$TEMP_DIR/strace_*.log" >/dev/null; then
        echo ""
        echo "Debug information available in: $TEMP_DIR/strace_*.log"
    fi
    exit 1
}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Error: $1 is required to run these checks"
        exit 1
    fi
}

# Prefer individual binaries: they are what CI builds, and tracing them avoids
# the multicall dispatch noise. Fall back to the multicall binary otherwise.
detect_binaries() {
    local util
    for util in $CHECK_UTILS; do
        if [ -f "$BIN_DIR/$util" ]; then
            echo "Using individual binaries"
            USE_MULTICALL=0
            return
        fi
    done

    if [ -f "$BIN_DIR/coreutils" ]; then
        echo "Using multicall binary"
        USE_MULTICALL=1
        COREUTILS_BIN="$BIN_DIR/coreutils"
        MULTICALL_UTILS=$("$COREUTILS_BIN" --list)
        return
    fi

    echo "Error: No binaries found. Please build first with 'cargo build --profile=$PROFILE'"
    exit 1
}

# Resolve a utility to a runnable command, or return 1 when it was not built.
# A multicall binary built without the unix feature set has no chmod or chown
# in it, so ask it what it actually dispatches.
util_cmd() {
    local util="$1"
    if [ "$USE_MULTICALL" -eq 1 ]; then
        grep -qx "$util" <<<"$MULTICALL_UTILS" || return 1
        echo "$COREUTILS_BIN $util"
    else
        [ -f "$BIN_DIR/$util" ] || return 1
        echo "$BIN_DIR/$util"
    fi
}

have_util() {
    util_cmd "$1" >/dev/null
}

detect_binaries
