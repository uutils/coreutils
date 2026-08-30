#!/bin/bash
#
# spell-checker:ignore strace reflink FDCWD
#
# Redundant-open verification.
#
# These strace-based checks assert that a utility resolves an operand path
# once and works off the resulting descriptor, rather than re-opening the
# same path for each stage of its work. Repeated opens cost syscalls and
# let two stages disagree about which file they are acting on.
#
# This is a correctness and efficiency check, not a security one: the
# companion scripts check-toctou.sh and check-safe-traversal.sh cover the
# cases where a split across two path-based syscalls is actually
# attacker-exploitable.
#

set -e

echo "=== Redundant Open Verification ==="

# shellcheck disable=SC2034  # read by check-common.sh once sourced
CHECK_UTILS="cp"
. "$(dirname "${BASH_SOURCE[0]}")/check-common.sh"

cd "$TEMP_DIR"

# cp used to open the source three times per copy: once in check_for_data,
# once in check_sparse_detection, then again in the copy itself. The
# sparseness probe and the copy now share one descriptor, so the strategy
# and the bytes come from the same file (#13185).
if cp_cmd=$(util_cmd cp); then
    echo ""
    echo "Testing cp (single source open)..."
    # rustix issues open(2) directly on Linux, so match both spellings --
    # an openat-only filter would silently match nothing and pass.
    for mode in "" "--reflink=never --sparse=always" "--reflink=never --sparse=never"; do
        rm -f cp_probe_dst
        printf 'source bytes for the probe\n' > cp_probe_src.txt
        # shellcheck disable=SC2086  # $mode is a deliberate word-split flag list
        strace -f -e trace=open,openat -o strace_cp_open.log \
            $cp_cmd $mode cp_probe_src.txt cp_probe_dst 2>/dev/null || true

        if [ ! -s strace_cp_open.log ]; then
            fail_immediately "strace produced no output for cp"
        fi
        opens=$(grep -cE '(open|openat)\(.*"cp_probe_src\.txt"' strace_cp_open.log || true)
        if [ "$opens" -ne 1 ]; then
            cat strace_cp_open.log
            fail_immediately "cp ${mode:-(default)} opened the source $opens times, expected exactly 1 - the sparseness probe and the copy must share one descriptor (#13185)"
        fi
        echo "✓ cp ${mode:-(default)} opens the source exactly once"
    done
    rm -f cp_probe_src.txt cp_probe_dst
fi

echo ""
echo "✓ Redundant open verification completed"
