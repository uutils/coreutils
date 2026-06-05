#!/bin/bash
#
# spell-checker:ignore mknod mknodat fchmod fchmodat mkfifoat strace
#
# TOCTOU (time-of-check / time-of-use) verification.
#
# These strace-based checks assert that utilities do not split a
# security-sensitive operation across two path-based syscalls (e.g. a
# stat() before open() that an attacker can race). The companion
# script check-safe-traversal.sh covers a different concern: that
# recursive walkers use the openat() family rather than re-resolving
# multi-component paths during traversal.
#

set -e

: ${PROFILE:=release-small}
export PROFILE

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMP_DIR=$(mktemp -d)

fail_immediately() {
    echo "❌ FAILED: $1"
    echo ""
    echo "Debug information available in: $TEMP_DIR/strace_*.log"
    exit 1
}

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo "=== TOCTOU Verification ==="

if [ -f "$PROJECT_ROOT/target/${PROFILE}/coreutils" ]; then
    echo "Using multicall binary"
    USE_MULTICALL=1
    COREUTILS_BIN="$PROJECT_ROOT/target/${PROFILE}/coreutils"
elif [ -f "$PROJECT_ROOT/target/${PROFILE}/mkfifo" ]; then
    echo "Using individual binaries"
    USE_MULTICALL=0
else
    echo "Error: No binaries found. Build first with 'cargo build --profile=${PROFILE}'"
    exit 1
fi

cd "$TEMP_DIR"

util_cmd() {
    if [ "$USE_MULTICALL" -eq 1 ]; then
        echo "$COREUTILS_BIN $1"
    else
        echo "$PROJECT_ROOT/target/${PROFILE}/$1"
    fi
}

if [ "$USE_MULTICALL" -eq 1 ]; then
    AVAILABLE_UTILS=$($COREUTILS_BIN --list)
else
    AVAILABLE_UTILS=""
    # The list intentionally holds a single util today; more will be added.
    # shellcheck disable=SC2043
    for util in mkfifo; do
        if [ -f "$PROJECT_ROOT/target/${PROFILE}/$util" ]; then
            AVAILABLE_UTILS="$AVAILABLE_UTILS $util"
        fi
    done
fi

# mkfifo must not call a path-based chmod after creating the FIFO: the
# second syscall would re-resolve the path and could be redirected by an
# attacker who swaps the FIFO for a symlink in between (issue #10020).
# After the fix, the kernel applies the requested mode atomically via
# mkfifo with cleared umask.
if echo "$AVAILABLE_UTILS" | grep -q "mkfifo"; then
    mkfifo_cmd=$(util_cmd mkfifo)
    rm -f test_fifo
    # mkfifo(3)/mkfifoat(3) are libc wrappers; the underlying syscall
    # is mknodat (or mknod on older kernels). Trace those plus any
    # chmod variants.
    strace -f -e trace=mknod,mknodat,chmod,fchmod,fchmodat,fchmodat2 \
        -o strace_mkfifo.log \
        $mkfifo_cmd -m 666 test_fifo 2>/dev/null || true

    if [ ! -s strace_mkfifo.log ]; then
        fail_immediately "strace produced no output for mkfifo"
    fi
    if ! grep -qE 'mknodat?\(' strace_mkfifo.log; then
        cat strace_mkfifo.log
        fail_immediately "mkfifo must call mknod/mknodat to create the FIFO"
    fi

    if grep -qE '\bchmod\([^,]*"test_fifo"' strace_mkfifo.log; then
        cat strace_mkfifo.log
        fail_immediately "mkfifo must not call path-based chmod after creation (issue #10020)"
    fi
    if grep -qE 'fchmodat2?\([^,]+, "test_fifo"' strace_mkfifo.log; then
        cat strace_mkfifo.log
        fail_immediately "mkfifo must not call fchmodat after creation (issue #10020)"
    fi
    echo "✓ mkfifo does not chmod after creation"
    rm -f test_fifo
fi

echo ""
echo "✓ TOCTOU verification completed"
