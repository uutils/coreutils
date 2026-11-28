#!/bin/bash
#
# spell-checker:ignore strace reflink FDCWD EXDEV tmpfs xdev dstdir RDONLY
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
CHECK_UTILS="cp mv"
. "$(dirname "${BASH_SOURCE[0]}")/check-common.sh"

# check-common.sh installs `trap cleanup EXIT`; extend it so the scratch
# directory on the second filesystem goes away with it.
SHM_DIR=""
cleanup() {
    rm -rf "$TEMP_DIR"
    if [ -n "$SHM_DIR" ]; then
        rm -rf "$SHM_DIR"
    fi
}

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

# cp -R used to look up each copied file 13 times and read the umask twice
# for it. Copy 10 and then 20 files: the difference is what one more file
# costs, without the fixed cost of starting up.
if cp_cmd=$(util_cmd cp); then
    echo ""
    echo "Testing cp -R (lookups per copied file)..."
    count_cp_tree() {
        rm -rf cp_tree cp_tree_dst
        mkdir cp_tree
        for i in $(seq "$1"); do : > "cp_tree/f$i"; done
        # shellcheck disable=SC2086  # $cp_cmd may be "coreutils cp"
        strace -f -e trace=%%stat,umask -o strace_cp_tree.log \
            $cp_cmd -R cp_tree cp_tree_dst 2>/dev/null || true
        if [ ! -s strace_cp_tree.log ]; then
            fail_immediately "strace produced no output for cp -R"
        fi
        stat_calls=$(grep -cE '\b[a-z0-9]*stat[a-z0-9]*\(' strace_cp_tree.log || true)
        umask_calls=$(grep -cE '\bumask\(' strace_cp_tree.log || true)
    }
    count_cp_tree 10
    stat_calls_10=$stat_calls
    umask_calls_10=$umask_calls
    count_cp_tree 20
    per_file_stats=$(( (stat_calls - stat_calls_10) / 10 ))
    per_file_umasks=$(( (umask_calls - umask_calls_10) / 10 ))
    if [ "$per_file_stats" -gt 7 ]; then
        cat strace_cp_tree.log
        fail_immediately "cp -R made $per_file_stats stat calls per copied file, expected at most 7"
    fi
    echo "✓ cp -R makes $per_file_stats stat calls per copied file"
    if [ "$per_file_umasks" -ne 0 ]; then
        fail_immediately "cp -R read the umask $per_file_umasks times per copied file, expected once for the whole copy"
    fi
    echo "✓ cp -R reads the umask once for the whole copy"
    rm -rf cp_tree cp_tree_dst
fi

# mv must not open an operand it is only going to rename, and its
# cross-device fallback -- a copy, since rename(2) cannot cross filesystems --
# must open the source once. mv has its own fallback rather than cp's
# (mv.rs, open_source + create_dest_restrictive + copy_fast), so this pins
# an invariant that a future move onto shared copy machinery could break.
if mv_cmd=$(util_cmd mv); then
    echo ""
    echo "Testing mv (single source open)..."

    printf 'moved by rename\n' > mv_probe_src.txt
    strace -f -e trace=open,openat -o strace_mv_rename.log \
        $mv_cmd mv_probe_src.txt mv_probe_dst 2>/dev/null || true
    if [ ! -s strace_mv_rename.log ]; then
        fail_immediately "strace produced no output for mv"
    fi
    opens=$(grep -cE '(open|openat)\(.*"mv_probe_src\.txt"' strace_mv_rename.log || true)
    if [ "$opens" -ne 0 ]; then
        cat strace_mv_rename.log
        fail_immediately "mv opened the source $opens times for a same-filesystem move, expected 0 - it must rename, not copy"
    fi
    echo "✓ mv (same filesystem) never opens the source"
    rm -f mv_probe_dst

    # /dev/shm is tmpfs on Linux, so it is a different filesystem from
    # $TEMP_DIR and rename(2) across the two returns EXDEV. Skip rather than
    # fail where that does not hold, so the check stays honest about what it
    # actually exercised.
    if [ -d /dev/shm ] && [ "$(stat -c %d /dev/shm)" != "$(stat -c %d "$TEMP_DIR")" ]; then
        SHM_DIR=$(mktemp -d /dev/shm/uutils-open-once.XXXXXX)

        printf 'moved across filesystems\n' > mv_probe_xdev.txt
        strace -f -e trace=open,openat -o strace_mv_xdev.log \
            $mv_cmd mv_probe_xdev.txt "$SHM_DIR/dst" 2>/dev/null || true
        opens=$(grep -cE '(open|openat)\(.*"mv_probe_xdev\.txt"' strace_mv_xdev.log || true)
        if [ "$opens" -ne 1 ]; then
            cat strace_mv_xdev.log
            fail_immediately "mv opened the source $opens times for a cross-device move, expected exactly 1"
        fi
        echo "✓ mv (cross-device file) opens the source exactly once"

        mkdir -p mv_probe_dir
        printf 'inner payload\n' > mv_probe_dir/inner.txt
        strace -f -e trace=open,openat -o strace_mv_xdev_dir.log \
            $mv_cmd mv_probe_dir "$SHM_DIR/dstdir" 2>/dev/null || true
        # Match any path ending in inner.txt opened O_RDONLY: that is the
        # source read, whether it is spelled relative to the cwd or, once the
        # traversal is fd-anchored, as a bare name under a directory fd. The
        # O_RDONLY rules out the destination, which is opened for writing.
        opens=$(grep -cE '(open|openat)\([^)]*"[^"]*inner\.txt"[^)]*O_RDONLY' strace_mv_xdev_dir.log || true)
        if [ "$opens" -ne 1 ]; then
            cat strace_mv_xdev_dir.log
            fail_immediately "mv opened a file inside a cross-device directory move $opens times, expected exactly 1"
        fi
        echo "✓ mv (cross-device directory) opens each file exactly once"
    else
        echo "- skipped mv cross-device checks: no second filesystem available"
    fi
    rm -rf mv_probe_src.txt mv_probe_xdev.txt mv_probe_dir
fi

echo ""
echo "✓ Redundant open verification completed"
