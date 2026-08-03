#!/bin/bash
#
# spell-checker:ignore mknod mknodat fchmod fchmodat mkfifoat strace newfstatat statx lstat CREAT FDCWD headtest
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
    for util in mkfifo touch head; do
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

# Test touch - creating a file must use O_CREAT but never O_TRUNC, so that a
# symlink planted in the metadata-check/open race window (#10019) is not
# truncated. This observes the flags directly, which integration tests cannot.
if echo "$AVAILABLE_UTILS" | grep -q "touch"; then
    echo ""
    echo "Testing touch (create_no_truncate)..."
    touch_cmd=$(util_cmd touch)
    strace -f -e trace=openat -o strace_touch_create.log $touch_cmd test_touch_new 2>/dev/null || true
    cat strace_touch_create.log
    if ! grep -q 'openat(.*test_touch_new.*O_CREAT' strace_touch_create.log; then
        fail_immediately "touch did not create test_touch_new via openat(O_CREAT)"
    fi
    if grep 'test_touch_new' strace_touch_create.log | grep -q 'O_TRUNC'; then
        fail_immediately "touch opened the target with O_TRUNC - vulnerable to truncating a symlink target (#10019)"
    fi
    echo "✓ touch creates with O_CREAT and without O_TRUNC"
    rm -f test_touch_new
fi

# Test head - the is-a-directory check must derive from the already-open
# descriptor (fstat/statx on the fd), not from a separate path-based stat
# performed before the open. A path stat followed by an open is a TOCTOU
# window (#11972): the object named by the path can be swapped in between.
if echo "$AVAILABLE_UTILS" | grep -q "head"; then
    echo ""
    echo "Testing head (fstat_after_open)..."
    head_cmd=$(util_cmd head)
    echo "headtest" > test_head_file.txt
    strace -f -e trace=openat,fstat,newfstatat,statx,stat,lstat \
        -o strace_head_metadata.log $head_cmd -c 4 test_head_file.txt 2>/dev/null || true
    cat strace_head_metadata.log
    if ! grep -q 'openat(AT_FDCWD, "test_head_file.txt"' strace_head_metadata.log; then
        fail_immediately "head did not open test_head_file.txt via openat"
    fi
    # The filename should appear only in the openat; any stat-family call naming
    # the path means head stat'd it before opening - the TOCTOU window.
    if grep '"test_head_file.txt"' strace_head_metadata.log | grep -qv 'openat('; then
        fail_immediately "head stat'd the path before opening it - TOCTOU window (#11972); metadata must come from the open descriptor"
    fi
    echo "✓ head reads metadata from the open descriptor, not a path stat"
    rm -f test_head_file.txt
fi

echo ""
echo "✓ TOCTOU verification completed"
